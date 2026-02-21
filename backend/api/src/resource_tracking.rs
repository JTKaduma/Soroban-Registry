use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const MAX_CPU: u64 = 100_000_000;
const MAX_MEM: u64 = 40 * 1024 * 1024;
const ALERT_PCT: f64 = 0.80;
const ALPHA: f64 = 0.3;
const BETA: f64 = 0.1;
const GAMMA: f64 = 0.2;
const SEASON_LEN: usize = 24;
const Z_P90: f64 = 1.2815515655446004;
const EPS: f64 = 1e-9;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_instructions: u64,
    pub mem_bytes: u64,
    pub storage_bytes: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageForecast {
    pub cpu_exhaustion_ts: Option<DateTime<Utc>>,
    pub mem_exhaustion_ts: Option<DateTime<Utc>>,
    pub cpu_exhaustion_ts_p90: Option<DateTime<Utc>>,
    pub mem_exhaustion_ts_p90: Option<DateTime<Utc>>,
    pub cpu_trend_per_sec: f64,
    pub mem_trend_per_sec: f64,
    pub confidence: f64,
    pub seasonal_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLimits {
    pub max_cpu_instructions: u64,
    pub max_mem_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAlert {
    pub metric: String,
    pub current_pct: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSummary {
    pub contract_id: String,
    pub current: ResourceUsage,
    pub history: Vec<ResourceUsage>,
    pub network_limits: NetworkLimits,
    pub alerts: Vec<ResourceAlert>,
    pub forecast: UsageForecast,
}

pub struct ResourceManager {
    data: HashMap<String, Vec<ResourceUsage>>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn record_usage(&mut self, contract_id: &str, usage: ResourceUsage) -> Vec<ResourceAlert> {
        let alerts = Self::check_alerts(&usage);
        self.data
            .entry(contract_id.to_string())
            .or_default()
            .push(usage);
        crate::metrics::RESOURCE_RECORDINGS.inc();
        if !alerts.is_empty() {
            crate::metrics::RESOURCE_ALERTS_FIRED.inc();
            for alert in &alerts {
                tracing::warn!(
                    contract_id = contract_id,
                    metric = alert.metric.as_str(),
                    current_pct = alert.current_pct,
                    message = alert.message.as_str()
                );
            }
        }
        alerts
    }

    fn check_alerts(u: &ResourceUsage) -> Vec<ResourceAlert> {
        let mut out = Vec::new();
        let cpu_pct = u.cpu_instructions as f64 / MAX_CPU as f64 * 100.0;
        if cpu_pct >= ALERT_PCT * 100.0 {
            out.push(ResourceAlert {
                metric: "cpu_instructions".into(),
                current_pct: (cpu_pct * 10.0).round() / 10.0,
                message: format!("CPU usage at {:.1}% of network limit", cpu_pct),
            });
        }
        let mem_pct = u.mem_bytes as f64 / MAX_MEM as f64 * 100.0;
        if mem_pct >= ALERT_PCT * 100.0 {
            out.push(ResourceAlert {
                metric: "mem_bytes".into(),
                current_pct: (mem_pct * 10.0).round() / 10.0,
                message: format!("Memory usage at {:.1}% of network limit", mem_pct),
            });
        }
        out
    }

    pub fn summary(&self, contract_id: &str) -> Option<ResourceSummary> {
        let history = self.data.get(contract_id)?;
        if history.is_empty() {
            return None;
        }
        let current = history.last().unwrap().clone();
        let alerts = Self::check_alerts(&current);
        let forecast = self.compute_forecast(history);
        crate::metrics::RESOURCE_FORECAST_RUNS.inc();
        Some(ResourceSummary {
            contract_id: contract_id.to_string(),
            current,
            history: history.clone(),
            network_limits: NetworkLimits {
                max_cpu_instructions: MAX_CPU,
                max_mem_bytes: MAX_MEM,
            },
            alerts,
            forecast,
        })
    }

    fn compute_forecast(&self, history: &[ResourceUsage]) -> UsageForecast {
        let cpu: Vec<f64> = history.iter().map(|u| u.cpu_instructions as f64).collect();
        let mem: Vec<f64> = history.iter().map(|u| u.mem_bytes as f64).collect();
        let last_ts = history.last().unwrap().timestamp;
        let current_cpu = *cpu.last().unwrap_or(&0.0);
        let current_mem = *mem.last().unwrap_or(&0.0);
        let dt = if history.len() >= 2 {
            let prev = history[history.len() - 2].timestamp;
            (last_ts - prev).num_seconds().max(1) as f64
        } else {
            3600.0
        };

        let cpu_deltas = deltas(&cpu);
        let mem_deltas = deltas(&mem);
        let (cpu_level, cpu_trend, cpu_seasonal) = holt_winters(&cpu_deltas);
        let (mem_level, mem_trend, mem_seasonal) = holt_winters(&mem_deltas);

        let mut cpu_step_burn = (cpu_level + cpu_trend).max(0.0) * cpu_seasonal.max(1.0);
        let mut mem_step_burn = (mem_level + mem_trend).max(0.0) * mem_seasonal.max(1.0);
        if cpu_step_burn <= EPS {
            cpu_step_burn = *cpu_deltas.last().unwrap_or(&0.0);
        }
        if mem_step_burn <= EPS {
            mem_step_burn = *mem_deltas.last().unwrap_or(&0.0);
        }

        let cpu_sigma = std_dev(&cpu_deltas);
        let mem_sigma = std_dev(&mem_deltas);
        let cpu_step_burn_p90 = (cpu_step_burn - Z_P90 * cpu_sigma).max(EPS);
        let mem_step_burn_p90 = (mem_step_burn - Z_P90 * mem_sigma).max(EPS);

        let cpu_exhaust = project_exhaustion(current_cpu, cpu_step_burn, MAX_CPU as f64, last_ts, dt);
        let mem_exhaust = project_exhaustion(current_mem, mem_step_burn, MAX_MEM as f64, last_ts, dt);
        let cpu_exhaust =
            project_exhaustion(current_cpu, cpu_step_burn, MAX_CPU as f64, last_ts, dt);
        let mem_exhaust =
            project_exhaustion(current_mem, mem_step_burn, MAX_MEM as f64, last_ts, dt);
        let cpu_exhaust_p90 =
            project_exhaustion(current_cpu, cpu_step_burn_p90, MAX_CPU as f64, last_ts, dt);
        let mem_exhaust_p90 =
            project_exhaustion(current_mem, mem_step_burn_p90, MAX_MEM as f64, last_ts, dt);

        let n = cpu_deltas.len().max(mem_deltas.len()) as f64;
        let variance_penalty = (cpu_sigma + mem_sigma) / (cpu_step_burn + mem_step_burn + 1.0);
        let confidence = ((1.0 - 1.0 / (n + 1.0)) * (1.0 - variance_penalty))
            .clamp(0.0, 0.99);
        let confidence = ((1.0 - 1.0 / (n + 1.0)) * (1.0 - variance_penalty)).clamp(0.0, 0.99);

        UsageForecast {
            cpu_exhaustion_ts: cpu_exhaust,
            mem_exhaustion_ts: mem_exhaust,
            cpu_exhaustion_ts_p90: cpu_exhaust_p90,
            mem_exhaustion_ts_p90: mem_exhaust_p90,
            cpu_trend_per_sec: cpu_step_burn / dt,
            mem_trend_per_sec: mem_step_burn / dt,
            confidence,
            seasonal_factor: cpu_seasonal,
        }
    }
}

fn holt_winters(data: &[f64]) -> (f64, f64, f64) {
    if data.is_empty() {
        return (0.0, 0.0, 1.0);
    }
    if data.len() == 1 {
        return (data[0], 0.0, 1.0);
    }

    let period = SEASON_LEN.min(data.len());
    let mut seasonal = vec![1.0_f64; period];
    if data.len() >= period {
        let avg = data[..period].iter().sum::<f64>() / period as f64;
        if avg > 0.0 {
            for i in 0..period {
                seasonal[i] = data[i] / avg;
            }
        }
    }

    let mut level = data[0];
    let mut trend = data[1] - data[0];
    let mut peak_seasonal = 1.0_f64;

    for (t, &y) in data.iter().enumerate().skip(1) {
        let s_idx = t % period;
        let s = seasonal[s_idx];
        let new_level = ALPHA * (y / s) + (1.0 - ALPHA) * (level + trend);
        let new_trend = BETA * (new_level - level) + (1.0 - BETA) * trend;
        if new_level > 0.0 {
            seasonal[s_idx] = GAMMA * (y / new_level) + (1.0 - GAMMA) * s;
        }
        if seasonal[s_idx] > peak_seasonal {
            peak_seasonal = seasonal[s_idx];
        }
        level = new_level;
        trend = new_trend;
    }

    (level, trend, peak_seasonal)
}

fn deltas(data: &[f64]) -> Vec<f64> {
    if data.len() < 2 {
        return vec![0.0];
    }
    data.windows(2).map(|w| (w[1] - w[0]).max(0.0)).collect()
}

fn std_dev(data: &[f64]) -> f64 {
    if data.len() <= 1 {
        return 0.0;
    }
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    let var = data
        .iter()
        .map(|x| {
            let d = x - mean;
            d * d
        })
        .sum::<f64>()
        / data.len() as f64;
    var.sqrt()
}

fn project_exhaustion(
    current: f64,
    burn_per_step: f64,
    limit: f64,
    base: DateTime<Utc>,
    dt: f64,
) -> Option<DateTime<Utc>> {
    if burn_per_step <= EPS {
        return None;
    }
    let remaining = limit - current;
    if remaining <= 0.0 {
        return Some(base);
    }
    let steps = remaining / burn_per_step;
    let secs = (steps * dt).max(0.0).round() as i64;
    if secs == 0 {
        return Some(base);
    }
    Some(base + chrono::Duration::seconds(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rising(n: usize) -> Vec<ResourceUsage> {
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        (0..n)
            .map(|i| ResourceUsage {
                cpu_instructions: ((i + 1) as u64) * (MAX_CPU / n as u64) - 1,
                mem_bytes: 1_000_000,
                storage_bytes: 512_000,
                timestamp: base + chrono::Duration::hours(i as i64),
            })
            .collect()
    }

    #[test]
    fn forecasts_cpu_exhaustion() {
        let mut mgr = ResourceManager::new();
        let hist = rising(48);
        for u in &hist {
            mgr.record_usage("c1", u.clone());
        }
        let summary = mgr.summary("c1").unwrap();
        assert!(summary.forecast.cpu_exhaustion_ts.is_some());
        assert!(summary.forecast.cpu_trend_per_sec > 0.0);
    }

    #[test]
    fn p90_is_not_earlier_than_expected() {
        let mut mgr = ResourceManager::new();
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        for i in 0..72_u64 {
            let noise = ((i % 5) * 25_000) as u64;
            mgr.record_usage(
                "cp90",
                ResourceUsage {
                    cpu_instructions: 1_000_000 + i * 1_200_000 + noise,
                    mem_bytes: 3_000_000 + i * 200_000,
                    storage_bytes: i * 1024,
                    timestamp: base + chrono::Duration::hours(i as i64),
                },
            );
        }
        let summary = mgr.summary("cp90").unwrap();
        assert!(summary.forecast.cpu_exhaustion_ts.is_some());
        assert!(summary.forecast.cpu_exhaustion_ts_p90.is_some());
        assert!(
            summary.forecast.cpu_exhaustion_ts.unwrap()
                <= summary.forecast.cpu_exhaustion_ts_p90.unwrap()
        );
    }

    #[test]
    fn alerts_at_eighty_percent() {
        let mut mgr = ResourceManager::new();
        let alerts = mgr.record_usage(
            "c2",
            ResourceUsage {
                cpu_instructions: (MAX_CPU as f64 * 0.81) as u64,
                mem_bytes: 10,
                storage_bytes: 0,
                timestamp: Utc::now(),
            },
        );
        assert!(!alerts.is_empty());
        assert_eq!(alerts[0].metric, "cpu_instructions");
    }

    #[test]
    fn seasonal_factor_tracks_peaks() {
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let mut mgr = ResourceManager::new();
        for i in 0..48_u64 {
            let phase = (i % 24) as f64 * std::f64::consts::PI / 12.0;
            let cpu = 20_000_000.0 + 15_000_000.0 * phase.sin();
            mgr.record_usage(
                "c3",
                ResourceUsage {
                    cpu_instructions: cpu as u64,
                    mem_bytes: 1_000_000,
                    storage_bytes: 0,
                    timestamp: base + chrono::Duration::hours(i as i64),
                },
            );
        }
        let summary = mgr.summary("c3").unwrap();
        assert!(summary.forecast.seasonal_factor > 1.0);
    }
}
