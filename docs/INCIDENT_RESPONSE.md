# Soroban Registry — Contract Incident Response Plan

**Version:** 1.0.0 | **Effective:** 2026-02-20 | **Owner:** Security Operations

---

## 1. Preparation

### Roles & Responsibilities

| Role | Primary | Backup |
|---|---|---|
| On-Call Engineer | Rotates weekly | Secondary on-call |
| Security Lead | `@security-lead` | `@security-lead-backup` |
| CTO / Exec Sponsor | `@cto` | `@vp-engineering` |

### Pre-Incident Checklist

- [ ] PagerDuty rotation configured and tested
- [ ] Runbook access verified for all on-call engineers
- [ ] Circuit-breaker CLI tooling (`soroban-registry incident`) installed on jump hosts
- [ ] Emergency contact list reviewed this quarter
- [ ] Monitoring alerts active (see `observability/` dashboards)

---

## 2. Detection

### Severity Classification

| Severity | Definition | SLA to Respond |
|---|---|---|
| **Critical** | Active exploit, fund loss risk, or data breach | **15 minutes** |
| **High** | Severe vulnerability with known PoC | 2 hours |
| **Medium** | Vulnerability without known exploit | 24 hours |
| **Low** | Minor bug, informational finding | 72 hours |

### Detection Sources

- Automated monitoring (Prometheus alerts / Grafana)
- External security researcher disclosure
- User-submitted bug reports
- Internal audit / penetration test

### Triggering an Incident

```bash
# Log a new incident and get an incident ID
soroban-registry incident trigger <contract_id> --severity <critical|high|medium|low>
```

> **Critical incidents automatically engage the off-chain circuit breaker**, halting
> registry interactions with the affected contract until the incident is resolved.

---

## 3. Containment

### Escalation Chain of Command

```
Detection Confirmed
       │
       ▼
On-Call Engineer (0–15 min)
 • Trigger incident via CLI
 • Validate circuit-breaker status
 • Begin preliminary root-cause analysis
       │
       ▼  (if Critical or High)
Security Lead (15–60 min)
 • Confirms scope and severity
 • Coordinates remediation team
 • Approves public disclosure timing
       │
       ▼  (if Critical with broad impact)
CTO / Exec Sponsor
 • Final authority on public statements
 • Regulatory / legal notification decisions
```

### Containment Actions by Severity

| Severity | Actions |
|---|---|
| Critical | Engage circuit breaker, freeze publisher account, isolate affected contracts, escalate immediately |
| High | Coordinated disclosure draft, patch fast-track via `soroban-registry patch create` |
| Medium | Schedule patch sprint, notify affected publishers privately |
| Low | Standard patch cycle |

### State Transitions

```
Detected → Responding → Contained → Recovered → PostReview
```

```bash
# Advance incident state
soroban-registry incident update <incident_id> --state <new_state>
```

---

## 4. Post-Incident Review

### Timeline (within 5 business days of resolution)

1. **Day 1–2** — Incident timeline reconstructed; root cause confirmed
2. **Day 3** — Blameless post-mortem meeting with all responders
3. **Day 4** — Action items assigned (backlog tickets created)
4. **Day 5** — Post-mortem document published internally; public summary published if warranted

### Post-Mortem Document Must Include

- Timeline of events (UTC timestamps)
- Root cause analysis (5-Whys or Fishbone)
- Systems impacted and user count
- Effectiveness of circuit breaker / containment
- Action items with owners and due dates

---

## 5. Public User Notification Template

Use the template below for GitHub Issues / status page updates.

---

```markdown
## [INCIDENT] <Short Title> — <Date UTC>

**Status:** [Investigating | Identified | Monitoring | Resolved]
**Severity:** [Critical | High | Medium | Low]
**Affected Contracts:** <contract IDs or "Under investigation">
**Incident ID:** <UUID from CLI>

### Summary

<One-paragraph plain-language description of what happened and what users may experience.>

### Impact

- <Describe observable impact (e.g., transactions may fail, read queries unaffected)>

### Actions Taken

- [HH:MM UTC] Incident detected via <source>
- [HH:MM UTC] Circuit breaker engaged for contract `<id>`
- [HH:MM UTC] Patch `<patch_id>` applied

### Next Update

We will post an update by **<ISO datetime UTC>** or sooner if the situation changes.

— Soroban Registry Security Operations
```
