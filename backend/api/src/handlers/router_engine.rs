use crate::handlers::models::NetworkMetadata;

pub fn select_optimal_network(networks: Vec<NetworkMetadata>) -> Option<NetworkMetadata> {
    networks
        .into_iter()
        .filter(|n| n.is_active)
        .max_by(|a, b| {
            let score_a = calculate_score(a);
            let score_b = calculate_score(b);
            score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn calculate_score(network: &NetworkMetadata) -> f32 {
    (network.reliability * 0.5) + 
    ((1.0 / (network.fee_avg as f32 + 1.0)) * 0.3) + 
    (network.throughput as f32 * 0.2)
}