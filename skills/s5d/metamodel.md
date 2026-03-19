# Metamodel Reference

Structural vocabulary for architectural decomposition. Every system decomposes into these artifacts. This document is normative: agent behavior, validation gates, and health scoring derive from the definitions below.

---

## Artifact Graph

```
Product (name, organization)
├── SuperSystem (kind: value_stream | jtbd)
│   ├── Role (kind: internal | external)
│   │   └── Concern (confirmed: bool)
│   │       └── AcceptanceMetric (units: percentage | number, how_to_measure)
│   └── AcceptanceMetric
├── Domain (classification REQUIRED, maturity_level, team)
│   ├── Capability (VerbNoun naming, domain REQUIRED)
│   │   ├── → Concern (addresses)
│   │   └── → Entity (operates on)
│   └── Entity (domain REQUIRED)
│       └── → Entity (relationship_type, cardinality, projection, aggregate_root)
├── SoftwareSystem
│   └── Container (technology)
│       └── Component (domain REQUIRED, feature REQUIRED, container REQUIRED, paths REQUIRED)
└── Feature (single-feature invariant: spec.id == feature.id)
    └── UseCase (scenario name only; steps live in verification.scenarios)
```

Edge: `Domain → Domain` via archetype (ohs | customer_supplier | acl | conformist | shared_kernel | published_language | partnership | separate_ways)

Transport: attached to Edge (`rest | grpc | messaging | graphql | websocket`)

---

## Spec File Shape (YAML)

Agents author YAML, not Rust field names. Use the serialized YAML shape below:

- Top-level keys: `s5d`, `id`, `version`, `product`, `tier`, `allow_update`, `meta`, `context`, `artifacts`, `links`, `contracts`, `gates`, `roc`, `problem`, `hypotheses`, `decision`, `note_rationale`, `expires_at`, `auto_noted`
- `decision`: For decision tier — written by `s5d decide` CLI. For all other tiers: must be null (runtime state lives in .record.yaml).
- Structural artifacts live under `artifacts:`
- Relationship tables live under `links:`
- Mutable lifecycle state lives in `.s5d/records/*.record.yaml`, not in the spec YAML

If approval, decision, gate, or reflection state looks "missing", inspect the matching `.record.yaml` before assuming the spec is wrong.

---

## Complete Artifact Definitions

### Product

Top-level artifact. One per specification.

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | non-empty, ≤64 chars, `[a-zA-Z0-9_.\-]` |
| `name` | string | human-readable label |
| `organization` | string | owning org |
| `description` | string | optional |
| `logo` | string | optional URL |

### SuperSystem

External context where the product operates. Defines the environment from which concerns and acceptance metrics emerge.

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | |
| `product` | string | parent product id |
| `name` | string | |
| `kind` | enum | `value_stream` or `jtbd` |
| `description` | string | optional |

`value_stream` — a repeatable end-to-end flow that delivers value (e.g., order-to-cash).
`jtbd` — a job the product helps a role accomplish (functional/social/emotional job framing).

### Role

Stakeholder or actor interacting with the system. Roles own concerns.

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | |
| `name` | string | |
| `kind` | enum | `internal` or `external` |
| `description` | string | optional |

### Concern

What a role cares about. May be confirmed (validated with real users) or hypothetical.

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | |
| `role` | string | parent role id |
| `name` | string | |
| `supersystem` | string? | optional supersystem ref — must reference declared supersystem |
| `confirmed` | bool | true = validated with real users |
| `description` | string | optional |

M2M: Concern ↔ AcceptanceMetric (a concern is measured by one or more metrics).

### AcceptanceMetric

Operationalizes a concern. Defines how satisfaction is measured.

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | |
| `name` | string | |
| `units` | enum | `percentage` or `number` |
| `how_to_measure` | string | concrete measurement procedure |
| `supersystem` | string? | optional — must reference declared supersystem |
| `description` | string | optional |

### Domain

Bounded context. The unit of strategic classification. NOT a crate or module — see Domain Decomposition Rules below.

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | |
| `product` | string | parent product id |
| `name` | string | |
| `team` | string? | optional owning team |
| `classification` | enum | `core`, `supporting`, or `generic` — **required** |
| `maturity_level` | enum | `genesis`, `custom`, `product`, or `commodity` |
| `description` | string | optional |

### Capability

What a domain CAN DO. Named in VerbNoun form. Capabilities are behavioral contracts — they describe actions, not data structures.

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | |
| `domain` | string | **required** — parent domain id |
| `name` | string | **required** — VerbNoun (e.g., `AuthorizePayment`, `TrackShipment`) |
| `description` | string | optional |

M2M: Capability → Concern (addresses) — which concerns this capability satisfies.
M2M: Capability → Entity (operates on) — which entities this capability reads/writes.

Invariant: `capability.domain` must match `component.domain` when a component binds to this capability.

### Entity

Domain data model. Struct, class, or aggregate. Entities live inside domains and must not leak across domain boundaries without translation.

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | |
| `domain` | string | **required** |
| `name` | string | **required** |
| `description` | string | optional |

M2M: Entity → Entity with `relationship_type`, `cardinality`, `projection`, `aggregate_root`.

#### Entity Relationship Fields

| Field | Type | Values |
|-------|------|--------|
| `cardinality` | enum | `1:1`, `1:N`, `M:N`, `1:0..1`, `1:0..N`, `N:0..N` |
| `projection` | bool | `true` = read model / materialized view of another entity |
| `aggregate_root` | bool | `true` = entry point for entity cluster; external access goes through this |

### SoftwareSystem

Deployable system boundary. Maps to a running application or service group.

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | |
| `name` | string | |
| `product` | string | parent product id |

### Container

Runtime unit within a system: process, database, queue, lambda, worker.

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | |
| `name` | string | |
| `system` | string | parent system id |
| `technology` | string | runtime technology label |

### Component

Code module implementing capabilities. All five structural fields are required — a component with missing bindings cannot be traced.

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | |
| `domain` | string | **required** — must match capability's domain |
| `feature` | string | **required** — parent feature id |
| `container` | string | **required** — parent container id |
| `name` | string | **required** |
| `paths` | `Vec<String>` | **required** — source paths relative to repo root |

Component has no `capabilities[]` or `entities[]` fields. Binding to capabilities and entities happens through the `links` section (`component_to_capability`, `component_to_entity` link kinds), not via direct fields on the component.

### Feature

Deliverable unit of work. One per spec file (single-feature invariant).

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | **must equal `spec.id`** |
| `product` | string | **required** |
| `name` | string | **required** |

Single-feature invariant: `features.len() == 1 AND features[0].id == spec.id`

### UseCase

User interaction scenario. Names the scenario; does not contain Given/When/Then steps.

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | |
| `feature` | string | **required** — parent feature id |
| `name` | string | **required** |

UseCase names the scenario only. Given/When/Then details live in `spec.verification.scenarios`, not in the UseCase artifact. UseCase connects to capabilities via links (see Feature → Capability relation below).

### Contract

API or schema contract artifact. High-tier work should declare at least one contract whenever an external or cross-service interface matters.

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | |
| `format` | enum | `openapi`, `json_schema`, `protobuf`, or `typespec` |
| `path` | string? | repo-relative file path to contract definition |
| `sha256` | string? | **required if `path` is set** — integrity check |
| `inline` | string? | inline contract text when no file exists yet |
| `binds_to` | list | **required** — non-empty bindings to capability ids and/or entity ids |

At least one of `path` or `inline` must be present. Contract checks validate declared contracts; they do not infer missing ones.

### Transport

Communication protocol attached to a domain edge.

| Field | Type | Values |
|-------|------|--------|
| `id` | string | |
| `type` | enum | `rest`, `grpc`, `messaging`, `graphql`, `websocket` |
| `serialization` | string? | optional |
| `description` | string? | optional |

### Edge

Cross-domain dependency with strategic classification.

| Field | Type | Notes |
|-------|------|-------|
| `from` | string | source domain id |
| `to` | string | target domain id |
| `archetype` | enum | see Domain Edge Archetypes below |
| `downstream_capability` | string? | optional capability ref |
| `transport_ref` | string? | optional transport id |
| `waiver` | object? | waiver with `reason`, `owner`, `expiry`, `plan` |

`links.edges[].waiver` is a structural edge waiver stored in the spec YAML. It is not the same as the runtime gate waiver recorded by `s5d_waiver`.

---

## Key Relations

### Feature → Capability

Feature does not relate directly to Capability. The path is:

```
Feature → UseCase → Capability
```

This traversal goes through links: `use_case_to_capability` link kind connects a UseCase to the Capability it exercises. Agents must follow this two-hop path when tracing feature coverage to capabilities.

---

## Domain Edge Archetypes

Eight archetypes cover every inter-domain relationship. Choosing the wrong archetype produces misleading health scores and incorrect cycle detection.

| Archetype | Direction | Definition | When to use |
|-----------|-----------|------------|-------------|
| `ohs` | upstream → downstream | Upstream publishes a stable, versioned API surface. Downstream consumes freely without negotiation. Upstream controls the contract unilaterally. | Upstream is shared by many downstream consumers and cannot negotiate per-consumer |
| `customer_supplier` | upstream → downstream | Upstream and downstream negotiate the contract. Downstream has input power but upstream makes final decisions. Explicit dependency: upstream must account for downstream needs in planning. | Two teams with direct relationship where downstream needs influence upstream roadmap |
| `acl` | downstream → upstream (ACL owned by downstream) | Downstream builds an isolating translation layer (Anti-Corruption Layer) between itself and the upstream model. Upstream is unaware. Downstream preserves its own model integrity at the cost of maintaining translation logic. | Upstream model is hostile, unstable, or conceptually misaligned with downstream domain |
| `conformist` | downstream adopts upstream model | Downstream surrenders modeling autonomy and adopts upstream's model as-is. Zero translation cost, maximum coupling. Acceptable only when upstream model is stable and semantically correct for downstream. | Integrating with a dominant external standard where the cost of ACL exceeds the cost of coupling |
| `shared_kernel` | bidirectional | An explicit, bounded subset of domain model is co-owned by two or more teams. Must be small. Changes require consultation from all owners. | Teams share a core concept (e.g., `Money`, `UserId`) that is genuinely identical in semantics |
| `published_language` | bidirectional | Standard interchange format (protobuf schema, JSON schema, OpenAPI spec). Defines the shape of data exchanged. Orthogonal to OHS: OHS describes access protocol, PL describes data format. | Any integration where the data shape must be explicitly versioned and documented |
| `partnership` | bidirectional | True mutual dependency. Both teams must coordinate releases. Neither team can ship a breaking change without the other. High synchronization cost. | Two domains that co-evolve a shared capability with no clear upstream/downstream |
| `separate_ways` | none (no integration) | Intentional decoupling. Integration cost has been evaluated and exceeds the benefit. Each context develops independently. | Domains that share a user journey but not data; full duplication is cheaper than coupling |

**Composition notes:**

- `ohs` + `published_language` compose: OHS describes *how* downstream accesses upstream (stable API set); PL describes *what* is exchanged (data format). They are independent axes and often co-occur.
- `acl` vs `conformist` is a cost decision: ACL preserves downstream model integrity at translation cost. Conformist is free upfront but accrues model debt as upstream evolves.
- `customer_supplier` ≠ `partnership`: In C/S, upstream can refuse downstream requests. In partnership, neither team can move unilaterally.
- Bidirectional archetypes (`shared_kernel`, `partnership`, `separate_ways`) are excluded from directed cycle detection.

---

## Domain Decomposition Rules

Domain = linguistic boundary where terms have one meaning. Crate/module = technical unit. Multiple crates may implement layers of one domain. The decomposition goal is to find minimal, coherent bounded contexts — not to mirror directory structure.

**Six-step collapse:**

1. **Merge layers.** `*-store` + `*-daemon` + `*-models` for the same noun → one domain. If a crate's entire role is data access or transport for another crate's structs, they belong to the same domain.

2. **Check ubiquitous language.** Same entity names with the same semantics across crates → same domain. An entity appearing as both a DB record and an API response object is a layering choice, not a domain split.

3. **Measure chattiness.** If crate A cannot serve a request without synchronously querying crate B, merge them. High import coupling between two units is a merge signal, not an interface signal.

4. **Evaluate shared kernel candidates.** A shared crate containing business logic used by two distinct domains → assess whether to merge all three or declare a formal shared_kernel. Shared utilities (logging, serialization, config) are not shared kernels — keep them `generic`.

5. **Split on polysemy.** Same term, different semantics in different contexts (`Account` = authentication record vs. billing ledger) → separate domains. Different stakeholder models operating on the same noun → separate.

6. **Isolate generic infrastructure.** Auth, observability, logging, config, rate-limiting → `generic`. Core business logic → `core`. Supporting enablers (notifications, admin UI, reporting) → `supporting`. `generic` must not depend on `core`.

Always show collapse reasoning explicitly: state which crates were merged into which domain and why.

---

## Domain Classification

| Class | Strategic definition |
|-------|---------------------|
| `core` | Unique business logic that differentiates this product. Cannot be bought off the shelf. Competitive moat. |
| `supporting` | Enables core domains to function. Not differentiating on its own. (Voice synthesis, admin tooling, config management.) |
| `generic` | Reusable across unrelated products. Commodity pattern. (Auth, logging, observability, rate-limiting.) |

Layering constraint: `core`(3) → `supporting`(2) → `generic`(1). A lower-classification domain depending on a higher-classification domain is a layering violation. `generic` must not import from `core`.

---

## Maturity Definitions

Maturity is a market property measured on two axes: **ubiquity** (how widespread the solution is) and **certainty** (how much consensus exists on how to solve the problem). Code-level signals are proxies — actual maturity is determined by market state, not by code quality.

| Stage | Strategic definition | Code-level signals |
|-------|---------------------|-------------------|
| `genesis` | Rare, constantly changing, poorly understood. No market, no standard solutions. Novel and uncertain. Organizations exploring this are taking strategic risk. | Prototype code, no tests, unstable API, experimental structure, frequent rewrites |
| `custom` | First working implementations exist. Each organization builds its own bespoke solution. Growing but scarce. Knowledge is accumulating but not yet standardized. | Working unique implementation, some tests, no published interface, not extractable |
| `product` | Multiple competing implementations exist. Feature competition between vendors. The problem is well-defined. Highest profitability window. Standard patterns are emerging. | Stable interface, tested, documented, could be extracted as a standalone library |
| `commodity` | Commonplace, standardized, cost of doing business. Ubiquitous and predictable. Utility-like. No competitive advantage in building your own. | Off-the-shelf solution used as-is, no custom implementation, vendor or OSS dependency |

Strategic implication: `genesis` domains require experimentation; `commodity` domains require procurement decisions, not engineering investment.

---

## Validation Rules

All rules enforced by `validate`. Violations block preview and approve.

### ID Format
- Non-empty
- ≤ 64 characters
- Characters: `[a-zA-Z0-9_.\-]` only

### Spec-Level
- `s5d` must be `"1.0"`
- Template `version` currently starts at `"1.0.0"`; treat it as the spec package version, not the protocol selector

### Tier: Note
- Must have `note_rationale`
- Early return after rationale check — no artifact validation

### Tier: Decision
- Must have `problem.signal`
- Must NOT have `artifacts` block
- Hypothesis IDs must be unique
- `hypotheses[].evidence[].formality`: integer 0–9 in raw YAML; command surfaces `s5d add-evidence --formality` / `s5d_add_evidence.formality` accept **1–5** only
- `hypotheses[].evidence[].congruence_level`: integer 0–3
- `confirmed_by` is required at `s5d_decide` time and is stored in the record file; `s5d_validate` does not enforce it

### Tier: Feature (Lightweight / Standard / High)
- Exactly 1 feature
- `spec.id == features[0].id`

### Tier: Standard and High
- `domains` list must be non-empty
- `capabilities` list must be non-empty
- `components` list must be non-empty

### Tier: Lightweight
- `capabilities` list must be non-empty

### Tier: High (additional)
- Spec `context` field must contain the word `"privacy"` — this is a keyword check. It ensures that privacy was explicitly considered when authoring the spec. Any mention of the word satisfies the gate; it does not parse or interpret the surrounding text.
- Declare at least one contract when the feature crosses an external or cross-service interface. Current contract checks validate declared contracts; they do not reject `contracts: []` by themselves.

### Domain Fields
- `classification`: must be one of `core`, `supporting`, `generic`
- `maturity_level`: must be one of `genesis`, `custom`, `product`, `commodity`

### Transport Fields
- `type`: must be one of `rest`, `grpc`, `messaging`, `graphql`, `websocket`

### Edge Fields
- `archetype`: must be one of `ohs`, `customer_supplier`, `conformist`, `acl`, `published_language`, `shared_kernel`, `partnership`, `separate_ways`

### Entity Relationship Fields
- `cardinality`: must be one of `1:1`, `1:N`, `M:N`, `1:0..1`, `1:0..N`, `N:0..N`
  Note: `1:1..1`, `1:1..N`, `M:1..N` are NOT valid — validation rejects them.

### Contract
- If `contract.path` exists, `contract.sha256` must also exist
- At least one of `contract.path` or `contract.inline` must exist
- `contract.format` must be one of `openapi`, `json_schema`, `protobuf`, `typespec`
- `contract.binds_to` must be non-empty

### Gate Kinds
- Valid kinds: `schema`, `graph`, `contract`, `lint`, `test`, `typecheck`, `policy`

### Concern/Metric Supersystem Reference
- If `concern.supersystem` is set, it must reference a declared supersystem id
- If `metric.supersystem` is set, it must reference a declared supersystem id

---

## Graph Validation

### Cycle Detection
- Uses Tarjan's SCC algorithm on all directed edges
- Reports ALL cycles, not just the first
- Bidirectional archetypes (`shared_kernel`, `partnership`, `separate_ways`) are excluded from the directed edge set before cycle detection runs
- A cycle among directed edges is a validation error

### Layering Check
- Numeric level: `core = 3`, `supporting = 2`, `generic = 1`, `unclassified = 0`
- A directed edge from a lower-level domain to a higher-level domain is a layering violation
- Example violation: `generic` domain depending on `core` domain
- Layering violations are reported separately from cycle violations

---

## Health Scoring

Health scoring quantifies architectural quality per domain and aggregates to a system-level score.

### Coupling Metrics (per domain)
- **Ca** (Afferent Coupling) = number of edges pointing TO this domain (incoming dependencies)
- **Ce** (Efferent Coupling) = number of edges pointing FROM this domain (outgoing dependencies)
- **Instability I** = `Ce / (Ca + Ce)`. If `Ca + Ce == 0`: `I = 0.0`
  - I → 0.0: stable (many dependents, few dependencies — good for `core`)
  - I → 1.0: unstable (no dependents, many dependencies — expected for leaf domains)

### Per-Domain Health Score
Starts at 100. Deductions are additive:

| Condition | Deduction |
|-----------|-----------|
| Domain is member of a directed cycle | −15 |
| Domain participates in a layering violation | −10 |
| Fragile: `I > 0.8 AND Ce > 3` | −5 |
| Hub: `Ca > 5` | −3 |
| Scattered: `Ce > 5` | −3 |

### Aggregate Health
`health(system) = min(health(domain) for domain in domains)`

This applies the weakest-link principle: system quality is bounded by its worst-performing domain. A single severely degraded domain brings the whole system score down.

### Delta Classification (score change between versions)
| Delta | Classification |
|-------|---------------|
| `> +5` | improved |
| `−5 .. +5` | stable |
| `−15 .. −5` | degraded |
| `< −15` | critical |

---

## Trace Confidence

Components bind to capabilities via `paths`. The confidence of that binding depends on how it was established.

| Method | Confidence | Mechanism |
|--------|-----------|-----------|
| Annotated | 0.95 | `@s5d` annotation found in source code at the path |
| Inferred | 0.70 | Symbol name in the file matches capability or entity name in the codebase index |
| Path-based | 0.50 | `component.paths` matches the file — no symbol or annotation match |

### Annotation Format

Any comment style is valid:

```
// @s5d:<artifact_kind>:<artifact_id>
# @s5d:<artifact_kind>:<artifact_id>
/* @s5d:<artifact_kind>:<artifact_id> */
-- @s5d:<artifact_kind>:<artifact_id>
```

Examples:
```rust
// @s5d:capability:cap.AuthorizePayment
// @s5d:entity:ent.Order
```

---

## Metamodel Invariants (Formal Notation)

```
// When domains are declared, every capability must belong to one of them
forall cap in capabilities where domains.len() > 0:
  cap.domain in domains

// Core domains cannot depend on Generic domains (layering)
forall edge in edges:
  NOT (classify(edge.from) == generic AND classify(edge.to) == core)

// Component-capability bindings must stay inside one domain
forall binding in links.component_to_capability:
  component(binding.component).domain == capability(binding.capability).domain

// Single-feature invariant
features.len() == 1 AND features[0].id == spec.id

// Aggregate health = weakest link
health(system) == min(health(domain) for domain in domains)

// No cycles in directed edges
forall cycle in SCC(directed_edges): cycle.len() <= 1
  where directed_edges = edges.filter(
    e => e.archetype NOT IN {shared_kernel, partnership, separate_ways}
  )

// Concern supersystem must be declared
forall c in concerns where c.supersystem != null:
  c.supersystem in supersystems

// Metric supersystem must be declared
forall m in metrics where m.supersystem != null:
  m.supersystem in supersystems

// Contract completeness
forall c in contracts where c.path != null:
  c.sha256 != null
```

---

## Five Specification Tiers

Tiers determine which artifacts are required and which gates are enforced.

| Tier | Artifacts Required | Default Gates | Non-waivable |
|------|-------------------|---------------|--------------|
| `note` | None | None | note_rationale present |
| `decision` | None (problem + hypotheses + evidence) | None | confirmed_by, no duplicate hypotheses |
| `lightweight` | capabilities | schema | schema gate |
| `standard` | domains + capabilities + components | schema, graph, lint | schema gate |
| `high` | domains + capabilities + components + `"privacy"` keyword in context field | schema, graph, lint, test, contract | schema gate, privacy keyword check |

**Note tier:** Just rationale. No artifact validation. Useful for capturing context without structural commitment.

**Decision tier:** Problem signal required. Must not contain an artifacts block. Hypotheses and evidence live in the spec YAML; human confirmation (`confirmed_by`) is captured when `s5d_decide` writes the record file. Evidence formality can be stored as 0–9 in raw YAML; command surfaces accept 1–5 (see fpf.md).

**Lightweight tier:** Minimal structural spec. Capabilities required so that component tracing is possible. Used for early feature shaping.

**Standard tier:** Full metamodel. Domains, capabilities, and components all required. Graph validation runs, catching cycle and layering violations.

**High tier:** Standard + privacy keyword required in spec `context` field + test and contract gates enforced. The privacy check is a keyword gate — validation scans for the literal word `"privacy"` to ensure it was considered. Used for features touching user data or compliance-sensitive paths.

---

## Minimal Valid YAML Examples

These are copy-paste templates. Fields marked optional can be omitted.

### Decision tier

```yaml
s5d: "1.0"
id: decision.auth-strategy
version: "1.0.0"
product: auth
tier: decision
allow_update: true
meta:
  title: auth-strategy
problem:
  signal: "Refresh-token path is stateful and p99 is unstable"
  constraints:
    - "Must preserve revocation semantics"
  targets:
    - "Reduce refresh latency"
  indicators:
    - "Token-family table size"
  acceptance: "Chosen variant preserves revocation and reduces p99 below 150ms"
  status: in_progress
hypotheses:
  - id: h1-server-rotation
    title: "Server-side rotation"
    content: "Persist token-family state and rotate on every refresh"
    scope: "auth boundary"
    kind: system
    layer: L1
    evidence:
      - id: 20260411-internal-h1-server-rotation
        type: internal
        content: "Staging benchmark keeps revocation lookup <5ms p95"
        verdict: pass
        formality: 5
        claim_scope: [latency, revocation-correctness]
        congruence_level: 2
        reliability: 0.8
contracts: []
gates: []
decision: null
auto_noted: false
```

### Lightweight tier

```yaml
s5d: "1.0"
id: feat-send-notification
version: "1.0.0"
product: shop
tier: lightweight
meta:
  title: send-notification
artifacts:
  products:
    - id: shop
      name: Shop
  features:
    - id: feat-send-notification
      product: shop
      name: Send Notification
  capabilities:
    - id: cap.send-notification
      domain: dom.notifications
      name: SendNotification
gates:
  - kind: schema
```

### Standard tier

```yaml
s5d: "1.0"
id: feat-authorize-payment
version: "1.0.0"
product: shop
tier: standard
meta:
  title: authorize-payment
artifacts:
  products:
    - id: shop
      name: Shop
  domains:
    - id: dom.payments
      product: shop
      name: Payments
      classification: core
      maturity_level: custom
  capabilities:
    - id: cap.authorize-payment
      domain: dom.payments
      name: AuthorizePayment
  features:
    - id: feat-authorize-payment
      product: shop
      name: Authorize Payment
  systems:
    - id: sys.api
      product: shop
      name: API
  containers:
    - id: cont.api
      system: sys.api
      name: api
      technology: rust-axum
  components:
    - id: comp.payment-handler
      domain: dom.payments
      feature: feat-authorize-payment
      container: cont.api
      name: PaymentHandler
      paths:
        - src/payments/handler.rs
links:
  feature_to_domain:
    - feature: feat-authorize-payment
      domain: dom.payments
  component_to_capability:
    - component: comp.payment-handler
      capability: cap.authorize-payment
gates:
  - kind: schema
  - kind: graph
  - kind: lint
```

### High tier delta

Add this on top of a standard spec when you need privacy-sensitive or contract-bound assurance:

```yaml
context: |
  ## Problem
  Sensitive identifiers cross a service boundary.
  ## Privacy
  Refresh-token family identifiers are confidential and must stay auditable.

contracts:
  - id: contract.refresh-api
    format: openapi
    path: contracts/refresh-api.yaml
    sha256: sha256:<digest>
    binds_to:
      - capability: <capability-id>

gates:
  - kind: schema
  - kind: graph
  - kind: lint
  - kind: test
  - kind: contract
```

---

## Phase Gates

The canonical flow is defined in SKILL.md: `Bootstrap → Frame → Decide → Spec → Build → Verify → Ship → Learn`. The gate system below operates within Build/Verify steps.

Structural gates block unconditionally. Methodological gates block by default but can be overridden with `--force`.

| Gate type | Override | Examples |
|-----------|----------|---------|
| Structural | Never (`--force` ignored) | `confirmed_by` required, preview before approve, no duplicate hypotheses, approval before import |
| Methodological | `--force` allowed | ≥3 hypotheses, evidence per hypothesis, acceptance criteria present, verifier ≠ approver |

**Trust separation:** The agent or person who approves a spec should differ from the agent or person who verifies it. Methodological check (overridable) but strongly recommended.

---

## SHA256 Chain

The chain ensures that what was approved matches what was imported.

1. **Preview** computes `diff_sha256` — a hash of all planned diff actions in canonical form:
   - Create: `C:<type>:<id>`
   - Link: `L:<link_description>`
   - Update: `U:<type>:<id>`
   - Delete: `D:<type>:<id>`

2. **Approve** binds `spec_sha256` (hash of the spec YAML) and `diff_sha256` into the approval record.

3. **Import** verifies both hashes match before writing to state. A mismatch means the spec or diff plan changed after approval — import is rejected.

4. **State fingerprint** = `hash(spec YAML + alias table entries)` — tracks whether the stored state drifts from the approved spec.

5. **Ledger** records: `spec_sha256`, `state_fingerprint`, `action` (`import` | `reconcile`), `timestamp`.

---

## Drift Detection

Drift occurs when the stored state diverges from the last approved and imported state.

| Status | Condition |
|--------|-----------|
| `synced` | Current `state_fingerprint` matches last successful ledger entry |
| `drifted` | Current `state_fingerprint` differs from last ledger entry |
| `degraded` | No baseline ledger entry exists — no import has completed successfully |

**Reconcile:** Re-imports the spec into state without requiring re-approval. Fixes alias table drift (IDs that have drifted in the alias table but whose spec content has not changed). Does not bypass the SHA256 chain — `spec_sha256` must still match the approved record.
