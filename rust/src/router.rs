use serde::Serialize;

/// S5D routing mode — prepare (frame first) or execute (spec → implement).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RouteMode {
    Prepare,
    Execute,
}

impl std::fmt::Display for RouteMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteMode::Prepare => write!(f, "prepare"),
            RouteMode::Execute => write!(f, "execute"),
        }
    }
}

/// S5D routing result — tier, mode, entry step, and reasoning.
#[derive(Debug, Clone, Serialize)]
pub struct RouteResult {
    /// Whether the request is in scope for S5D at all.
    pub in_scope: bool,
    /// Tier (None if out of scope).
    pub tier: Option<crate::Tier>,
    /// Mode (None if out of scope).
    pub mode: Option<RouteMode>,
    /// Entry step number (None if out of scope).
    pub entry_step: Option<u8>,
    /// Waiver text if any steps are auto-waived.
    pub waiver: Option<String>,
    /// Human-readable reason for the routing decision.
    pub reason: String,
}

impl std::fmt::Display for RouteResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.in_scope {
            return write!(f, "Route: out-of-scope\nReason: {}", self.reason);
        }
        let tier = self.tier.as_ref().map(|t| t.to_string()).unwrap_or_default();
        let mode = self.mode.as_ref().map(|m| m.to_string()).unwrap_or_default();
        let step = self.entry_step.unwrap_or(0);
        write!(f, "Route: tier={}, mode={}, entry=Step {}", tier, mode, step)?;
        if let Some(waiver) = &self.waiver {
            write!(f, "\nWaiver: {}", waiver)?;
        }
        write!(f, "\nReason: {}", self.reason)
    }
}

// ── Signal keywords ──────────────────────────────────────────────────────────

const OUT_OF_SCOPE_SIGNALS: &[&str] = &[
    "bugfix",
    "bug fix",
    "fix typo",
    "typo",
    "config change",
    "config-only",
    "docs-only",
    "documentation only",
    "spec status",
];

/// Patterns like "fix <small-thing>" that are likely out of scope.
const FIX_SMALL_SIGNALS: &[&str] = &[
    "off-by-one",
    "null check",
    "nil check",
    "npe",
    "null pointer",
    "missing return",
    "wrong index",
    "fence post",
    "fencepost",
    "one-liner",
];

const DECISION_SIGNALS: &[&str] = &[
    "should we",
    "how should",
    "which approach",
    "compare",
    "tradeoff",
    "trade-off",
    "evaluate options",
    "architecture decision",
    "adr",
    "choose between",
    "decision",
    " vs ",
    " versus ",
];

const HIGH_SIGNALS: &[&str] = &[
    "auth",
    "authentication",
    "authorization",
    "payment",
    "billing",
    "security",
    "pii",
    "compliance",
    "gdpr",
    "hipaa",
    "encryption",
    "credential",
    "secret",
    "token",
    "oauth",
];

const EXECUTE_SIGNALS: &[&str] = &[
    "implement",
    "build",
    "add feature",
    "create",
    "wire up",
    "ship",
];

const MULTI_DOMAIN_SIGNALS: &[&str] = &[
    "across",
    "multiple domains",
    "cross-domain",
    "end-to-end",
    "integration",
    "2+ domains",
    "multi-domain",
];

/// Classify a request description into a routing decision.
///
/// Rule-based, top-to-bottom, first match wins.
pub fn route(description: &str) -> RouteResult {
    let lower = description.to_lowercase();

    // 1. Out of scope
    for signal in OUT_OF_SCOPE_SIGNALS {
        if lower.contains(signal) {
            return RouteResult {
                in_scope: false,
                tier: None,
                mode: None,
                entry_step: None,
                waiver: None,
                reason: format!("matched out-of-scope signal: \"{}\"", signal),
            };
        }
    }

    // Small fix patterns — but only if no high-risk signal is present.
    // "fix null pointer in auth token validation" should route high, not out-of-scope.
    let has_high_signal = HIGH_SIGNALS.iter().any(|s| lower.contains(s));
    if !has_high_signal {
        for signal in FIX_SMALL_SIGNALS {
            if lower.contains(signal) {
                return RouteResult {
                    in_scope: false,
                    tier: None,
                    mode: None,
                    entry_step: None,
                    waiver: None,
                    reason: format!("small fix pattern: \"{}\"", signal),
                };
            }
        }
    }

    // 2. Tier selection — first match wins, ordered by priority
    let tier = if DECISION_SIGNALS.iter().any(|s| lower.contains(s)) {
        crate::Tier::Decision
    } else if HIGH_SIGNALS.iter().any(|s| lower.contains(s)) {
        crate::Tier::High
    } else if MULTI_DOMAIN_SIGNALS.iter().any(|s| lower.contains(s)) {
        crate::Tier::Standard
    } else {
        // Default: lightweight for single-domain features
        crate::Tier::Lightweight
    };

    // 3. Mode selection
    let mode = if EXECUTE_SIGNALS.iter().any(|s| lower.contains(s)) {
        RouteMode::Execute
    } else {
        RouteMode::Prepare // default — safer to frame first
    };

    // 4. Entry point + waiver
    let (entry_step, waiver) = match (&tier, &mode) {
        (crate::Tier::Decision, _) => (1, None),
        (_, RouteMode::Prepare) => (1, None),
        (_, RouteMode::Execute) => (
            3,
            Some("WAIVER: Steps 1-2 | Reason: architecture decided | Approved: router".into()),
        ),
    };

    // Build reason
    let tier_reason = match &tier {
        crate::Tier::Decision => "decision signals detected",
        crate::Tier::High => "touches security/auth/payment/compliance",
        crate::Tier::Standard => "multi-domain scope detected",
        crate::Tier::Lightweight => "single-domain feature (default)",
        _ => "classified",
    };
    let mode_reason = match &mode {
        RouteMode::Execute => "explicit implementation intent",
        RouteMode::Prepare => "default — frame before building",
    };

    RouteResult {
        in_scope: true,
        tier: Some(tier),
        mode: Some(mode),
        entry_step: Some(entry_step),
        waiver,
        reason: format!("{}; {}", tier_reason, mode_reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bugfix_out_of_scope() {
        let r = route("bugfix: off-by-one in parser");
        assert!(!r.in_scope);
    }

    #[test]
    fn test_typo_out_of_scope() {
        let r = route("Fix typo in README");
        assert!(!r.in_scope);
    }

    #[test]
    fn test_decision_tier() {
        let r = route("How should we handle caching for the API?");
        assert!(r.in_scope);
        assert_eq!(r.tier, Some(crate::Tier::Decision));
        assert_eq!(r.entry_step, Some(1));
    }

    #[test]
    fn test_high_tier_auth() {
        let r = route("Add OAuth2 authentication flow");
        assert!(r.in_scope);
        assert_eq!(r.tier, Some(crate::Tier::High));
    }

    #[test]
    fn test_high_tier_payment() {
        let r = route("Implement payment processing with Stripe");
        assert!(r.in_scope);
        assert_eq!(r.tier, Some(crate::Tier::High));
    }

    #[test]
    fn test_standard_multi_domain() {
        let r = route("Cross-domain event pipeline for orders and inventory");
        assert!(r.in_scope);
        assert_eq!(r.tier, Some(crate::Tier::Standard));
    }

    #[test]
    fn test_lightweight_default() {
        let r = route("Add a search filter to the dashboard");
        assert!(r.in_scope);
        assert_eq!(r.tier, Some(crate::Tier::Lightweight));
    }

    #[test]
    fn test_execute_mode() {
        let r = route("Implement user profile page");
        assert_eq!(r.mode, Some(RouteMode::Execute));
        assert_eq!(r.entry_step, Some(3));
        assert!(r.waiver.is_some());
    }

    #[test]
    fn test_prepare_mode_default() {
        let r = route("User profile page needs redesign");
        assert_eq!(r.mode, Some(RouteMode::Prepare));
        assert_eq!(r.entry_step, Some(1));
        assert!(r.waiver.is_none());
    }

    #[test]
    fn test_decision_always_step1() {
        // Even with execute signal, decision tier always starts at Frame
        let r = route("Implement: how should we handle caching?");
        assert_eq!(r.tier, Some(crate::Tier::Decision));
        assert_eq!(r.entry_step, Some(1));
    }

    #[test]
    fn test_high_overrides_multi_domain() {
        // Security signal takes priority over multi-domain
        let r = route("Cross-domain authentication service");
        assert_eq!(r.tier, Some(crate::Tier::High));
    }

    #[test]
    fn test_display_in_scope() {
        let r = route("Implement cross-domain pipeline");
        let s = r.to_string();
        assert!(s.contains("tier=standard"));
        assert!(s.contains("mode=execute"));
        assert!(s.contains("Step 3"));
    }

    #[test]
    fn test_display_out_of_scope() {
        let r = route("bugfix: null pointer");
        let s = r.to_string();
        assert!(s.contains("out-of-scope"));
    }

    #[test]
    fn test_fix_small_out_of_scope() {
        let r = route("fix off-by-one in pagination");
        assert!(!r.in_scope, "fix off-by-one should be out of scope");

        let r = route("add null check to user lookup");
        assert!(!r.in_scope, "add null check should be out of scope");
    }

    #[test]
    fn test_should_we_decision() {
        let r = route("should we use Redis or Postgres for session storage?");
        assert_eq!(r.tier, Some(crate::Tier::Decision));
    }

    #[test]
    fn test_vs_decision() {
        let r = route("Redis vs Postgres for caching");
        assert_eq!(r.tier, Some(crate::Tier::Decision));
    }

    #[test]
    fn test_high_signal_overrides_small_fix() {
        // "null pointer" is a small-fix signal, but "auth" is high-risk — high wins
        let r = route("fix null pointer in auth token validation");
        assert!(r.in_scope, "high-risk signal should override small-fix");
        assert_eq!(r.tier, Some(crate::Tier::High));
    }
}
