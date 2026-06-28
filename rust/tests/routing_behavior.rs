//! Synthetic situational tests for the deterministic router (`s5d route`).
//!
//! Captures the behavioral battery run during the skill-hardening assessment.
//! Two groups:
//!   - `router_classifies_clean_requests` — behavior the keyword classifier gets
//!     right; a regression lock on well-formed English requests.
//!   - `router_known_limitations` — cases the keyword-only classifier gets WRONG
//!     (non-English, "X and Y" multi-domain, out-of-scope-beats-high). S5D routes
//!     LLM-first: the agent reads the request semantically and uses `s5d route`
//!     only as a cross-check, so these gaps are ACCEPTED and pinned here as
//!     documented behavior. If the router is ever made semantic/multilingual,
//!     update these assertions deliberately — they are a tripwire, not a target.

use s5d::{route, RouteMode, Tier};

fn assert_route(prompt: &str, in_scope: bool, tier: Option<Tier>, mode: Option<RouteMode>) {
    let r = route(prompt);
    assert_eq!(r.in_scope, in_scope, "in_scope for {prompt:?}");
    assert_eq!(r.tier, tier, "tier for {prompt:?}");
    assert_eq!(r.mode, mode, "mode for {prompt:?}");
}

#[test]
fn router_classifies_clean_requests() {
    use RouteMode::{Execute, Prepare};
    use Tier::{Decision, High, Lightweight};

    // decision tier — explicit tradeoff/choice
    assert_route(
        "Should we use Redis or Postgres for sessions?",
        true,
        Some(Decision),
        Some(Prepare),
    );
    assert_route(
        "design the caching layer: write-through vs write-behind",
        true,
        Some(Decision),
        Some(Prepare),
    );
    // high tier — auth/payment/security/PII
    assert_route(
        "Implement OAuth2 login across web and mobile",
        true,
        Some(High),
        Some(Execute),
    );
    assert_route(
        "Migrate the billing database to a new schema",
        true,
        Some(High),
        Some(Prepare),
    );
    assert_route(
        "encrypt PII at rest for the profiles domain",
        true,
        Some(High),
        Some(Prepare),
    );
    assert_route(
        "build an end-to-end checkout flow with inventory and payments",
        true,
        Some(High),
        Some(Execute),
    );
    // "checkout" is a payment-adjacent high signal in its own right — the
    // keyword router now catches behavior-probe P4 ("shopping cart and checkout
    // flow") instead of leaning entirely on the LLM cross-check.
    assert_route(
        "Add a shopping cart and checkout flow",
        true,
        Some(High),
        Some(Prepare),
    );
    // lightweight default — single-domain feature
    assert_route(
        "Add a search filter to the dashboard",
        true,
        Some(Lightweight),
        Some(Prepare),
    );
    assert_route(
        "ship the new onboarding screen",
        true,
        Some(Lightweight),
        Some(Execute),
    );
    // out of scope — small fixes, no S5D
    assert_route("fix typo in the payment receipt email", false, None, None);
    assert_route("add null check to user lookup", false, None, None);
}

#[test]
fn execute_waiver_is_a_non_authoritative_suggestion() {
    // The router is a cross-check, not the source of truth
    // (decision.s5d-routing-philosophy). On an execute-intent request it records a
    // Target+Decide auto-waiver, but it MUST NOT claim a decision was made or
    // approved — it has verified none. Lock the honest framing so a future edit
    // can't quietly restore the authoritative "architecture decided / Approved:
    // router" wording the conclave flagged as a control-plane-bypass shape.
    let waiver = route("ship the new onboarding screen")
        .waiver
        .expect("execute intent records a Target+Decide auto-waiver");
    assert!(
        waiver.contains("WAIVER: Target+Decide"),
        "auto-waiver concept preserved: {waiver:?}"
    );
    assert!(
        !waiver.contains("architecture decided"),
        "router must not claim architecture was decided: {waiver:?}"
    );
    assert!(
        !waiver.contains("Approved: router"),
        "router must not claim approval authority: {waiver:?}"
    );
    assert!(
        waiver.contains("suggestion") && waiver.contains("verify"),
        "router must frame its waiver as a suggestion to verify: {waiver:?}"
    );

    // Prepare mode records no waiver at all.
    assert!(
        route("Add a search filter to the dashboard")
            .waiver
            .is_none(),
        "prepare mode emits no waiver"
    );
}

#[test]
fn router_known_limitations() {
    use RouteMode::{Execute, Prepare};
    use Tier::Lightweight;

    // (1) Non-English blindness. The classifier is English-keyword-only, so a
    // Russian request meaning "add shopping cart and checkout" — where checkout
    // implies payment, i.e. high tier — falls through to the lightweight default.
    // The reasoning agent routes this correctly to high; `s5d route` does not.
    // LLM-primary routing is the compensator.
    assert_route(
        "добавить корзину покупок и оформление заказа",
        true,
        Some(Lightweight),
        Some(Prepare),
    );

    // (2) Multi-domain via "X and Y" conjunction is not detected — only literal
    // "cross-domain"/"multi-domain"/"end-to-end"/"integration" are. Two domains
    // (notifications + orders) read as a single-domain lightweight feature.
    assert_route(
        "create a notifications service and wire it to orders",
        true,
        Some(Lightweight),
        Some(Execute),
    );

    // (3) Out-of-scope beats high-risk. The high-signal override guards only the
    // small-fix patterns, not the out-of-scope list, so a config change to an
    // AUTH timeout exits S5D even though it touches a security path. A human/LLM
    // should pull this back in when the auth surface is material.
    assert_route(
        "config change for the auth service timeout",
        false,
        None,
        None,
    );
}
