//! Suite: compiled analyzers for DDD modeling and scaling anti-patterns.
//!
//!
//! Critical contract: when the repo's detected stacks do not match any stack
//! that a skill's checks cover, `analyze` returns `status: StackNotCovered`,
//! `scanned_files` = 0, and an empty findings list. The CLI renders an explicit
//! note so that "0 findings" is never silently indistinguishable from "clean pass".

pub mod ddd;
pub mod flatten;
pub mod scaling;
pub mod scan;

/// Severity ordered ascending so `>=` comparisons work naturally.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    /// Case-insensitive parse; unknown names map to `Info` rather than failing.
    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "critical" => Self::Critical,
            "high" => Self::High,
            "medium" => Self::Medium,
            "low" => Self::Low,
            _ => Self::Info,
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => f.write_str("info"),
            Self::Low => f.write_str("low"),
            Self::Medium => f.write_str("medium"),
            Self::High => f.write_str("high"),
            Self::Critical => f.write_str("critical"),
        }
    }
}

/// One analysis finding, paired with fix + validate instructions.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Finding {
    pub check: String,
    pub severity: Severity,
    pub path: String,
    pub detail: String,
    pub fix: String,
    pub validate: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimension: Option<String>,
}

/// One signal from the `detect` step.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DetectSignal {
    pub id: String,
    pub present: bool,
    pub evidence: String,
}

/// Whether the skill's checks covered any of the detected stacks.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoverageStatus {
    Scanned,
    StackNotCovered,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Summary {
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub total: usize,
}

impl Summary {
    pub fn from_findings(findings: &[Finding]) -> Self {
        let mut s = Self {
            high: 0,
            medium: 0,
            low: 0,
            total: findings.len(),
        };
        for f in findings {
            match f.severity {
                Severity::High | Severity::Critical => s.high += 1,
                Severity::Medium => s.medium += 1,
                Severity::Low => s.low += 1,
                Severity::Info => {}
            }
        }
        s
    }
}

/// Result of `analyze`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AnalysisReport {
    pub root: String,
    /// Number of files the checks actually inspected (0 when StackNotCovered).
    pub scanned_files: usize,
    pub stacks: Vec<String>,
    /// True when the repo walk hit the file cap — coverage is partial.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
    pub status: CoverageStatus,
    pub findings: Vec<Finding>,
    pub summary: Summary,
}

/// Result of `detect`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DetectReport {
    pub root: String,
    pub scanned_files: usize,
    pub stacks: Vec<String>,
    /// True when the repo walk hit the file cap — coverage is partial.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
    pub signals: Vec<DetectSignal>,
    pub summary: DetectSummary,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DetectSummary {
    pub present: usize,
    pub signals_total: usize,
}
