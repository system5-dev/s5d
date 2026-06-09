//! `skill` subcommand handlers: ddd and scaling analyzers + flatten.
//!
//! Thin presentation wrappers over `s5d::suite::{ddd, scaling, flatten}`.
//! When analyze returns StackNotCovered the JSON is still emitted (the
//! status field carries the contract) but a note is written to stderr.

pub fn run_skill_ddd_detect(root: &std::path::Path) -> anyhow::Result<()> {
    let report = s5d::suite::ddd::detect(root)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

pub fn run_skill_ddd_analyze(
    root: &std::path::Path,
    do_flatten: bool,
    min_severity: s5d::suite::Severity,
) -> anyhow::Result<()> {
    let report = s5d::suite::ddd::analyze(root)?;

    if report.status == s5d::suite::CoverageStatus::StackNotCovered {
        let stacks = report.stacks.join(", ");
        eprintln!(
            "note: stack not covered by deterministic checks (detected: {}; covered: typescript, javascript)",
            if stacks.is_empty() { "none".to_string() } else { stacks }
        );
    }

    let json = serde_json::to_string_pretty(&report)?;
    if do_flatten {
        println!("{}", flatten_or_not_covered(&report, &json, "ddd-refactor", min_severity)?);
    } else {
        println!("{}", json);
    }
    Ok(())
}

pub fn run_skill_scaling_detect(root: &std::path::Path) -> anyhow::Result<()> {
    let report = s5d::suite::scaling::detect(root)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

pub fn run_skill_scaling_analyze(
    root: &std::path::Path,
    do_flatten: bool,
    min_severity: s5d::suite::Severity,
) -> anyhow::Result<()> {
    let report = s5d::suite::scaling::analyze(root)?;

    if report.status == s5d::suite::CoverageStatus::StackNotCovered {
        let stacks = report.stacks.join(", ");
        eprintln!(
            "note: stack not covered by deterministic checks (detected: {}; covered: typescript, javascript)",
            if stacks.is_empty() { "none".to_string() } else { stacks }
        );
    }

    let json = serde_json::to_string_pretty(&report)?;
    if do_flatten {
        println!(
            "{}",
            flatten_or_not_covered(&report, &json, "scaling-review", min_severity)?
        );
    } else {
        println!("{}", json);
    }
    Ok(())
}

// A StackNotCovered report flattened to "0 anomaly(ies)" would recreate the
// false-clean this port exists to kill — the verbatim flatten channel must
// carry the not-covered verdict itself, not rely on a stderr note.
fn flatten_or_not_covered(
    report: &s5d::suite::AnalysisReport,
    json: &str,
    label: &str,
    min_severity: s5d::suite::Severity,
) -> anyhow::Result<String> {
    if report.status == s5d::suite::CoverageStatus::StackNotCovered {
        let stacks = if report.stacks.is_empty() {
            "none".to_string()
        } else {
            report.stacks.join(", ")
        };
        return Ok(format!(
            "## {} — stack not covered by deterministic checks (detected: {}; covered: typescript, javascript)\n⚠ no files scanned — this is NOT a clean verdict",
            label, stacks
        ));
    }
    s5d::suite::flatten::flatten(json, label, min_severity)
}

pub fn run_skill_flatten(label: &str, min_severity: s5d::suite::Severity) -> anyhow::Result<()> {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    let md = s5d::suite::flatten::flatten(&buf, label, min_severity)?;
    println!("{}", md);
    Ok(())
}

/// Parse a severity string for CLI args, defaulting to Medium on unknown input.
pub fn parse_severity(s: &str) -> s5d::suite::Severity {
    s5d::suite::Severity::from_str_lossy(s)
}
