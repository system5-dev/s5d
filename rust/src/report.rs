use crate::models::*;
use crate::project::S5dProject;
use chrono::{DateTime, Duration, Utc};

pub struct ReportData {
    pub total_specs: usize,
    pub by_status: Vec<(String, usize)>,
    pub by_tier: Vec<(String, usize)>,
    pub leading: LeadingMetrics,
    pub lagging: LaggingMetrics,
    pub anti: AntiMetrics,
    pub learn: LearnMetrics,
}

pub struct LearnMetrics {
    pub operated_count: usize,
    pub total_heuristics: usize,
    pub open_follow_ups: usize,
}

pub struct LeadingMetrics {
    pub new_specs_30d: usize,
    pub approval_rate: f64,
    pub avg_gates_per_spec: f64,
}

pub struct LaggingMetrics {
    pub applied_rate: f64,
    pub synced_rate: f64,
}

pub struct AntiMetrics {
    pub lightweight_overuse: f64,
    pub stale_specs: usize,
    pub drift_rate: f64,
}

pub fn compute_report(project: &S5dProject) -> anyhow::Result<ReportData> {
    let specs = project.discover_specs()?;
    let now = Utc::now();
    let thirty_days_ago = now - Duration::days(30);
    let stale_threshold = now - Duration::days(14);

    let total = specs.len();
    let mut status_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut tier_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut applied_count = 0usize;
    let mut synced_count = 0usize;
    let mut drifted_count = 0usize;
    let mut lightweight_count = 0usize;
    let mut stale_count = 0usize;
    let mut total_gates = 0usize;
    let mut approved_count = 0usize;
    let mut new_30d = 0usize;

    for (path, spec) in &specs {
        let filename = path.file_name().unwrap().to_string_lossy();
        let record = project.load_record(&filename)?;

        let tier_str = format!("{}", spec.tier);
        *tier_counts.entry(tier_str).or_default() += 1;

        if let Some(ref r) = record {
            let status_str = format!("{}", r.status);
            *status_counts.entry(status_str).or_default() += 1;

            match r.status {
                SpecStatus::Applied => {
                    applied_count += 1;
                    match r.sync_status {
                        SyncStatus::Synced => synced_count += 1,
                        SyncStatus::Drifted | SyncStatus::Degraded => drifted_count += 1,
                        _ => {}
                    }
                }
                SpecStatus::Approved => approved_count += 1,
                SpecStatus::Proposed => {
                    if let Some(first) = r.status_history.first() {
                        if let Ok(ts) = DateTime::parse_from_rfc3339(&first.timestamp) {
                            if ts < stale_threshold {
                                stale_count += 1;
                            }
                        }
                    }
                }
                _ => {}
            }

            total_gates += r.gate_results.len();

            if let Some(first) = r.status_history.first() {
                if let Ok(ts) = DateTime::parse_from_rfc3339(&first.timestamp) {
                    if ts >= thirty_days_ago {
                        new_30d += 1;
                    }
                }
            }
        } else {
            *status_counts.entry("unknown".into()).or_default() += 1;
        }

        if matches!(spec.tier, Tier::Lightweight) {
            lightweight_count += 1;
        }
    }

    approved_count += applied_count;

    let approval_rate = if total > 0 {
        approved_count as f64 / total as f64
    } else {
        0.0
    };
    let avg_gates = if total > 0 {
        total_gates as f64 / total as f64
    } else {
        0.0
    };
    let applied_rate = if total > 0 {
        applied_count as f64 / total as f64
    } else {
        0.0
    };
    let synced_rate = if applied_count > 0 {
        synced_count as f64 / applied_count as f64
    } else {
        0.0
    };
    let lw_rate = if total > 0 {
        lightweight_count as f64 / total as f64
    } else {
        0.0
    };
    let drift_rate_val = if applied_count > 0 {
        drifted_count as f64 / applied_count as f64
    } else {
        0.0
    };

    // Learn metrics
    let mut operated_total = 0usize;
    let mut heuristic_total = 0usize;
    let mut follow_up_total = 0usize;

    for (path, _spec) in &specs {
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        if let Some(ref r) = project.load_record(&filename)? {
            if r.status == SpecStatus::Operated {
                operated_total += 1;
                if let Some(ref refl) = r.reflection {
                    heuristic_total += refl.heuristics.len();
                    follow_up_total += refl.follow_ups.len();
                }
            }
        }
    }

    let mut by_status: Vec<(String, usize)> = status_counts.into_iter().collect();
    by_status.sort_by(|a, b| b.1.cmp(&a.1));
    let mut by_tier: Vec<(String, usize)> = tier_counts.into_iter().collect();
    by_tier.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(ReportData {
        total_specs: total,
        by_status,
        by_tier,
        leading: LeadingMetrics {
            new_specs_30d: new_30d,
            approval_rate,
            avg_gates_per_spec: avg_gates,
        },
        lagging: LaggingMetrics {
            applied_rate,
            synced_rate,
        },
        anti: AntiMetrics {
            lightweight_overuse: lw_rate,
            stale_specs: stale_count,
            drift_rate: drift_rate_val,
        },
        learn: LearnMetrics {
            operated_count: operated_total,
            total_heuristics: heuristic_total,
            open_follow_ups: follow_up_total,
        },
    })
}

pub fn render_report(data: &ReportData) -> String {
    let now = Utc::now().format("%Y-%m-%d %H:%M UTC");
    let mut out = String::new();

    out.push_str(&format!("# S5D Adoption Report\n\nGenerated: {}\n\n", now));

    out.push_str(&format!(
        "## Overview\n\nTotal specs: {}\n\n",
        data.total_specs
    ));

    out.push_str("### By Status\n\n| Status | Count |\n|---|---|\n");
    for (status, count) in &data.by_status {
        out.push_str(&format!("| {} | {} |\n", status, count));
    }

    out.push_str("\n### By Tier\n\n| Tier | Count |\n|---|---|\n");
    for (tier, count) in &data.by_tier {
        out.push_str(&format!("| {} | {} |\n", tier, count));
    }

    out.push_str("\n## Leading Indicators (30d)\n\n");
    out.push_str(&format!("- New specs: {}\n", data.leading.new_specs_30d));
    out.push_str(&format!(
        "- Approval rate: {:.0}%\n",
        data.leading.approval_rate * 100.0
    ));
    out.push_str(&format!(
        "- Avg gates/spec: {:.1}\n",
        data.leading.avg_gates_per_spec
    ));

    out.push_str("\n## Lagging Indicators (90d)\n\n");
    out.push_str(&format!(
        "- Applied rate: {:.0}%\n",
        data.lagging.applied_rate * 100.0
    ));
    out.push_str(&format!(
        "- Synced rate (of applied): {:.0}%\n",
        data.lagging.synced_rate * 100.0
    ));

    out.push_str("\n## Anti-Metrics\n\n");
    out.push_str(&format!(
        "- Lightweight overuse: {:.0}%",
        data.anti.lightweight_overuse * 100.0
    ));
    if data.anti.lightweight_overuse > 0.8 {
        out.push_str(" warning");
    }
    out.push('\n');
    out.push_str(&format!(
        "- Stale specs (>14d proposed): {}",
        data.anti.stale_specs
    ));
    if data.anti.stale_specs > 0 {
        out.push_str(" warning");
    }
    out.push('\n');
    out.push_str(&format!(
        "- Drift rate: {:.0}%",
        data.anti.drift_rate * 100.0
    ));
    if data.anti.drift_rate > 0.1 {
        out.push_str(" warning");
    }
    out.push('\n');

    out.push_str("\n## Learn\n\n");
    out.push_str(&format!(
        "- Operated specs: {}\n",
        data.learn.operated_count
    ));
    out.push_str(&format!(
        "- Heuristics captured: {}\n",
        data.learn.total_heuristics
    ));
    out.push_str(&format!(
        "- Follow-ups pending: {}\n",
        data.learn.open_follow_ups
    ));

    out
}
