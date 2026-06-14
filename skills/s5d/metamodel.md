# Metamodel Reference

Structural vocabulary for architectural decomposition. Every system decomposes into these artifacts. Validation gates derive from the definitions below.

---

## Abstract Metamodel

The S5D abstract metamodel is a graph of typed concepts and explicit relations between them. Each concept has UUID identity. Rust structures in [`rust/src/models.rs`](../../rust/src/models.rs) are a projection of this graph; YAML spec shape (next section) is the authored surface.

### Concepts

| Concept | Belongs to | Required attrs | Optional attrs |
|---|---|---|---|
| **Product** | Organization (external boundary) | `name`, `organization` | `description`, `logo` |
| **Domain** | Product, Team (external boundary) | `name`, `product`, `team` | `description`, `classification`, `maturity_level` |
| **Capability** | Domain | `name`, `domain` | `description` |
| **Entity** | Domain | `name`, `domain` | `description` |
| **SuperSystem** | Product | `name`, `kind`, `product` | `description` |
| **Role** | (free-standing stakeholder) | `name`, `kind` | `description` |
| **Concern** | Role, optionally SuperSystem | `name`, `role` | `description`, `confirmed`, `supersystem` |
| **AcceptanceMetric** | optionally SuperSystem | `name`, `units` | `description`, `how_to_measure`, `supersystem` |
| **DomainLevelDiagram** | Product | `product`, `data`, `revision` | — |
| **KnowledgeProvenance** | any concept (polymorphic ref) | `origin_type`, `status`, `context_type`, `target_concept_id` | `raw_artifact`, `recognized_contexts` |

S5D extends with implementation-side concepts: **Feature** (`spec.id == feature.id`), **UseCase**, **SoftwareSystem / Container / Component** (C4-like), **Contract**, **Gate**, **Hypothesis**, **HypothesisEvidence**, **DecisionRecord**, **Reflection** — see "Spec File Shape" below.

### Enums

| Enum | Values | Used by |
|---|---|---|
| `DomainClassification` | `Generic`, `Supporting`, `Core` | Domain.classification |
| `DomainArchetype` | `Partnership`, `Shared Kernel`, `Customer/Supplier Development`, `Conformist`, `Anticorruption Layer`, `Open-host Service`, `Published Language`, `Separate Ways`, `Big Ball of Mud` | Domain→Capability edge.archetype |
| `MaturityLevel` (Wardley) | `Genesis`, `Custom`, `Product`, `Commodity` | Domain.maturity_level |
| `RelationShipType` | `1:0..1`, `1:1..1`, `1:0..N`, `1:1..N`, `N:0..N`, `M:1..N` | Entity↔Entity edge.relationship_type |
| `SuperSystemKind` | `Value Stream`, `JTBD` | SuperSystem.kind |
| `RoleKind` | `Internal`, `External` | Role.kind |
| `Units` | `Percentage`, `Number` | AcceptanceMetric.units |
| `ContextType` | `Product`, `Architecture`, `Marketing`, `Organization`, `Other` | KnowledgeProvenance.context_type |
| `OriginType` | `Manual`, `Generated from External Definition`, `Generated from System Input` | KnowledgeProvenance.origin_type |
| `OriginStatus` | `Draft`, `Confirmed` | KnowledgeProvenance.status |

### Relations — Composition (ownership)

```
Organization (ext)
  └─◇ Product
        ├─◇ Domain ──◊ Team (ext)
        │     ├─◇ Capability
        │     └─◇ Entity
        ├─◇ SuperSystem
        ├─◇ DomainLevelDiagram        (versioned snapshot)
        └── KnowledgeProvenance       (polymorphic ref to any concept)

Role (free-standing)
  └─◇ Concern ──◊ SuperSystem (optional)

SuperSystem
  └─◇ AcceptanceMetric (optional ownership)
```

`◇` = composition (cascade delete). `◊` = association (link, no ownership).

### Relations — M2M (with edge attributes)

Each row is an edge type carrying its own data on the connection itself (not on either endpoint).

| Edge | Endpoints | Edge attrs | Semantics |
|---|---|---|---|
| **Domain → Capability** (downstream) | upstream Domain → downstream Capability | `archetype: DomainArchetype` | DDD context-mapping pattern between bounded contexts. The downstream Domain is implied via the target Capability's owning Domain. |
| **Capability ↔ Entity** | Capability ↔ Entity | (no extra attrs) | A capability operates on entities; can be read as message/request-response. |
| **Capability ↔ Concern** | Capability ↔ Concern | (no extra attrs) | A capability addresses a concern. |
| **Entity → Entity** | Entity → Entity (self-ref, directional) | `relationship_type: RelationShipType`, `projection: bool`, `aggregate_root: bool` | Entity-knows-about-Entity (not symmetric). `projection=true` → read model; `aggregate_root=true` → DDD aggregate. |
| **Concern ↔ AcceptanceMetric** | Concern ↔ AcceptanceMetric | `confirmed: bool` | A metric applied to a concern; `confirmed` tracks whether acceptance criteria have been met for this pairing. |

### Uniqueness & Lifecycle Constraints

- Product `(name, organization)` is unique — product names unique per org.
- Domain `(name, product)` is unique — domain names unique per product.
- SuperSystem `(name, product)` is unique — supersystem names unique per product.
- Concern `(role, name)` is unique — same role can't raise two concerns with the same name.
- AcceptanceMetric `(name, supersystem)` is unique — metric names unique per supersystem.
- DomainLevelDiagram `revision` is auto-incremented per Product. Append-only versioning.
- Composition is cascade-delete: removing an owner (Product / Domain / SuperSystem / Role) removes its composed concepts.

### Knowledge Provenance (cross-cutting)

`KnowledgeProvenance` attaches via polymorphic reference to **any** concept and carries: who/what produced it (`OriginType`), where it stands in the lifecycle (`OriginStatus: Draft | Confirmed`), what kind of context (`ContextType`), and optional source artifact + structured postprocessing output. This is the metamodel's record of ML/LLM-generated provenance — distinct from FPF's evidence chain in S5D's Hypothesis records.

### Projection Status

Already projected into `rust/src/models.rs`: Product, Domain, Capability, Entity, SuperSystem, Role, Concern, AcceptanceMetric, plus all enums above. **Not yet projected**: `DomainLevelDiagram` (versioned snapshot), `KnowledgeProvenance` (polymorphic ref), per-edge identities (s5d uses simpler `Edge` + `Link` records without their own UUIDs). Adding them is mechanical when needed; defer until a use case requires them.

---

## Spec File Shape (YAML)

Agents author YAML, not Rust field names. Use the serialized YAML shape below:

- Top-level keys: `s5d`, `id`, `version`, `product`, `tier`, `allow_update`, `meta`, `context`, `workflow`, `artifacts`, `links`, `contracts`, `gates`, `roc`, `problem`, `hypotheses`, `decision`, `note_rationale`, `expires_at`, `auto_noted`
- `decision`: For decision tier — written by `s5d decide` CLI. For all other tiers: must be null (runtime state lives in .record.yaml).
- Structural artifacts live under `artifacts:`
- Optional workflow/process-support data lives under `workflow:`
- Relationship tables live under `links:`
- Mutable lifecycle state lives in `.s5d/records/*.record.yaml`, not in the spec YAML

If approval, decision, gate, or reflection state looks "missing", inspect the matching `.record.yaml` before assuming the spec is wrong.

### Workflow Shell

`workflow` is optional. It lets S5D support an existing process without pretending to become the process itself.

Current fields:

| Field | Type | Notes |
|-------|------|-------|
| `mode` | enum | `research`, `plan`, `implement`, `operate` |
| `target_architecture.summary` | string | required if `target_architecture` exists |
| `target_architecture.invariants` | list | optional |
| `delivery_strategy.summary` | string | required if `delivery_strategy` exists |
| `delivery_strategy.rationale` | string | optional |
| `role_map` | map | role name → owner / agent type |
| `review_policy.cross_model_required` | bool | optional |
| `review_policy.required_on` | list | optional |
| `execution_mode.engine` | enum | `manual` or `ralph` |
| `execution_mode.stop_conditions` | list | required if `execution_mode` exists |
| `phases[]` | list | optional phase plan entries |

Each `workflow.phases[]` entry requires:

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | unique within the workflow |
| `title` | string | non-empty |
| `scope` | string | non-empty |
| `roles` | list | non-empty |
| `acceptance` | list | non-empty |
| `rollback` | list | non-empty |

Reflection in `.record.yaml` may now additionally record:
- `verdict`: `confirmed`, `refuted`, `inconclusive`, `iterate`, `kill`
- `measurement_window`: free-form measurement window string
- `telemetry_refs[]`: dashboard/query/log references backing the verdict

External phase runs are configured in `.s5d/config.yaml`, not in the spec:

| Field | Type | Notes |
|-------|------|-------|
| `engines.<name>.approved` | bool | required to be true before `s5d phase run` executes it |
| `engines.<name>.command` | list | argv template, not a shell string; supports `{spec}`, `{spec_filename}`, `{phase}`, `{engine}`, `{run_id}`, `{run_dir}`, `{stdout}`, `{stderr}`, `{s5d_dir}`, `{root}` |
| `engines.<name>.reasoning` | string | optional label recorded with the run |

`s5d phase run` captures stdout/stderr under `.s5d/runs/`, appends `phase_runs[]` to the record, and appends a `verified` phase-history entry only when the command exits successfully. Human phase acceptance remains a separate step.

### Mandate Envelope (autonomous loop)

`mandate` is an optional top-level field that authorizes a long autonomous loop over the spec's phases. A human admits it **once** (`s5d mandate admit`, SHA-bound to the spec); the agent then drives the loop with `s5d mandate run` (one control step per cycle), which adjudicates and returns the next phase or halts. s5d adjudicates and authorizes only — it never runs an engine itself; the agent executes the returned phase and re-invokes.

| Field | Type | Notes |
|-------|------|-------|
| `scope` | string | non-empty — what the loop may touch (paths/domains) |
| `budget` | Roc | bounds the loop; **must** set `max_calls` and/or `max_time_s` (unbounded autonomy is rejected) |
| `min_gate_floor` | list | eligibility gates that must have a recorded pass before each iteration; each must be declared in `gates[]` |
| `stop_conditions` | list | free-form loop-end conditions |

Validation (enforced at `mandate admit`): rejected on `high`/`decision` tiers (those stay human-gated per action), on any invalid spec, on empty `scope`, on an unbounded `budget`, and on any `min_gate_floor` entry not declared in `gates[]`. Each authorized `mandate run` step increments `mandate_iterations` in the record; the loop halts (escalates) when a gate floor has no recorded pass, on detected drift, budget exhaustion, unreadable time-budget admission, or a spec edit that breaks the admission SHA binding — and completes when all phases are accepted.

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
| `paths` | `Vec<String>` | **required** — source paths relative to repo root; absolute paths, parent traversal (`..`), and null bytes are invalid |

Component has no `capabilities[]` or `entities[]` fields. Binding to capabilities and entities happens through the `links` section (`component_to_capability`, `component_to_entity` link kinds), not via direct fields on the component.

`s5d trace <path>` / `s5d_trace` uses `component.paths` as the entry point, then follows `component_to_capability` and decision `hypotheses[].spec_ref` links. If a source file should be governed by S5D and trace returns no match, the owning spec is missing a component path claim.

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

Eight archetypes cover every inter-domain relationship. Choosing the wrong archetype produces incorrect cycle detection and layering violations.

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
- If `workflow.mode` exists, it must be one of `research`, `plan`, `implement`, `operate`
- If `workflow.execution_mode.engine` exists, it must be `manual` or `ralph`
- `workflow.phases[].id` must be unique
- `workflow.phases[]` entries require non-empty `title`, `scope`, `roles`, `acceptance`, and `rollback`

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
- Effective gates default to `review` for Decision tier when `gates:` is empty. The built-in handler passes only when ≥1 evidence record carries `evidence_type=gate:review` and `verdict=pass` on any hypothesis. Reviewers (human or agent) attach their result via `s5d add-evidence --evidence-type gate:review`.

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
- Effective gates default to `schema`, `graph`, and `review` for High tier when `gates:` is empty. The `review` handler requires ≥1 `evidence_type=gate:review` with `verdict=pass`.

### Gate: review (built-in)
Available kind in `gates[]` and in the effective tier defaults. Built-in handler scans `hypotheses[].evidence[]` for any record with `evidence_type=gate:review` (or `gate:review:<sub>`) and `verdict=pass`. Passes when count ≥ 1. Decision and High tiers use this gate by default when `gates:` is empty; other tiers may add it explicitly. External reviewers (tribunal, reviewer agent, human) record their findings as evidence — S5D does not invoke them, only verifies that review evidence exists.

### FPF-Aligned Fields (Round-1)

S5D records carry FPF-derived fields. Cite module ids when authoring or reviewing; cards under `skills/s5d/references/fpf/cards/` index full module text.

**Hypothesis (FPF B.5.2:13.3 prime-hypothesis L0 record).** `hypothesis.prompt` — explicit question this hypothesis answers (cite of `problem.signal`). `hypothesis.next_move` — typed next move: `deduction|probe|build|defer`. Validator-enforced enum on Decision tier. CLI: `s5d add-hypothesis --prompt ... --next-move ...`.

**Evidence (FPF C.2:4.2 Δ-moves).** `evidence.refine_kind` — required when `verdict=refine`. Allowed: `formalise|generalise|specialise|calibrate|validate|congrue`. Validator rejects `verdict=refine` without `refine_kind` on Decision tier. CLI: `s5d add-evidence --refine-kind ...`.

**DecisionRecord (FPF C.11 Decsn-CAL + C.18 NQD).** `decision_subject` (who/what is the decision about), `decision_subject_granularity` (system|component|module|line|...), `evaluative_surface` (named axes + policy), `belief_state` (what was assumed), `outcome_model` (what is predicted), `pareto_set` (Vec of non-dominated hypothesis IDs, distinct from `rejected_ids`), `choice_rule` (e.g. "lex-order(thinness>auditability)", "policy:minimize-coupling"). CLI: `s5d decide --decision-subject ... --pareto-set ... --choice-rule ...`. **FPF C.18 forbids weighted scalarization** — `evaluative_surface` containing markers like `weighted sum`, `0.5*`, `scalar fold` is rejected by the validator unless `--force` is passed.

**ProblemCard (FPF C.17:14 Anti-Goodhart).** `problem.goodhart_guard` — free-form prose naming which `indicators` are observation-only (not optimization targets), so the agent cannot game them by accident.

**GateResult (FPF B.3.4:5 CC-ED.5 + C.22:5.4 Eligibility/Acceptance).** `gate_result.waiver_expires_at` — RFC3339 timestamp; required when `status=waived`. `s5d import` auto-revokes waivers past their expiry. `s5d_waiver` MCP tool defaults to `now+90d` when `expires_at` is omitted. `gate_result.kind_class` — `eligibility` (admission, schema/graph/architecture by default) or `acceptance` (threshold). `s5d run-gates` runs eligibility gates first; on failure, fast-fails remaining acceptance gates.

**Record (FPF A.3.3:9.3 conformance/drift).** `record.drift_tolerance` — free-form policy string like `"schema=block, code=block, doc=warn"`. Allowed actions per artifact: `block|warn|allow`. `s5d drift-check` returns tri-state `{Synced|Drifted|Partial|Degraded}`: when policy contains only `warn|allow` rules and no `block`, drift is reported as `Partial` (does not block import) instead of `Drifted`. Default (no policy): binary behavior preserved.

### Lifecycle ↔ FPF ADI Mapping

S5D's lifecycle stages map onto the FPF reasoning kernel:

| S5D stage | FPF kernel | Cards |
|---|---|---|
| Explore | Abduction (FPF B.5.2 — generate prime hypotheses with prompts, real rivals, filters) | `B.5.2`, `B.5.2.0`, `B.5.2.1` |
| Shape | Deduction (FPF C.11/C.18 — derive predictions, declare evaluative surface, hold Pareto front) | `C.11`, `C.18`, `C.2` |
| Evidence | Induction (FPF B.3 — close loop with corroboration/refutation; F-G-R triad with min-rules across chain) | `B.3`, `B.3.4`, `C.2` |
| Operate | Evolution Loop (FPF B.4 — Observe→Refine→Deploy with B.3 assurance metrics + Anti-Goodhart guards from C.17) | `B.4`, `C.17`, `B.3.4` |

**Discipline:** No induction without prior deduction. No deduction without prior abduction. Skipping a stage is a methodological waiver — record the reason, name the responsible authority, and set a short-term expiry (`s5d waiver --expires-at`).

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
- Valid kinds: `schema`, `graph`, `architecture`, `review`, `contract`, `lint`, `test`, `typecheck`, `policy`
- `architecture` is built in: it validates the spec/graph first, checks that `components[].paths` stay under the repo root and resolve to source files, rejects overlapping component ownership, and requires cross-domain source imports to be represented by `links.edges`.
- `review` is built in: it validates that review evidence exists (`evidence_type=gate:review`, `verdict=pass`).
- `.s5d/codebase/modules.yaml` and `.s5d/codebase/coverage.yaml` are optional coverage snapshots. `s5d codebase sync` rebuilds them from source files and component paths; `s5d codebase check` fails when the snapshot is stale.

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

## Metamodel Invariants

```
forall cap in capabilities where domains.len() > 0:
  cap.domain in domains

forall edge in edges:
  NOT (classify(edge.from) == generic AND classify(edge.to) == core)

forall binding in links.component_to_capability:
  component(binding.component).domain == capability(binding.capability).domain

forall component in components where capabilities.len() > 0:
  exists binding in links.component_to_capability where binding.component == component.id

forall path in components[].paths:
  path is relative to repo root AND path does not contain '..' or null bytes

features.len() == 1 AND features[0].id == spec.id

forall cycle in SCC(directed_edges): cycle.len() <= 1
  where directed_edges = edges.filter(
    e => e.archetype NOT IN {shared_kernel, partnership, separate_ways}
  )

forall c in concerns where c.supersystem != null:
  c.supersystem in supersystems

forall m in metrics where m.supersystem != null:
  m.supersystem in supersystems

forall c in contracts where c.path != null:
  c.sha256 != null
```

---

## Five Specification Tiers

Tiers determine which artifacts are required and which gates are enforced.

| Tier | Artifacts Required | Default Gates | Non-waivable |
|------|-------------------|---------------|--------------|
| `note` | None | None | note_rationale present |
| `decision` | None (problem + hypotheses + evidence) | review | confirmed_by, no duplicate hypotheses |
| `lightweight` | capabilities | schema | schema gate |
| `standard` | domains + capabilities + components | schema, graph | schema gate |
| `high` | domains + capabilities + components + `"privacy"` keyword in context field | schema, graph, review | schema gate, privacy keyword check |

**Note tier:** Just rationale. No artifact validation. Useful for capturing context without structural commitment.

**Decision tier:** Problem signal required. Must not contain an artifacts block. Hypotheses and evidence live in the spec YAML; human confirmation (`confirmed_by`) is captured when `s5d_decide` writes the record file. Evidence formality can be stored as 0–9 in raw YAML; command surfaces accept 1–5.

**Lightweight tier:** Minimal structural spec. Capabilities required so that component tracing is possible. Used for early feature shaping.

**Standard tier:** Full metamodel. Domains, capabilities, and components all required. Graph validation runs, catching cycle and layering violations.

**High tier:** Standard + privacy keyword required in spec `context` field + a built-in `review` gate that passes only on recorded review evidence (`s5d decision add-evidence <spec> --hypothesis-id <id> --evidence-type gate:review --verdict pass`). `test`/`contract` gates run only when declared in the spec AND configured in `.s5d/config.yaml` — they are not enforced by default. The privacy check is a keyword gate — validation scans for the literal word `"privacy"` to ensure it was considered. Used for features touching user data or compliance-sensitive paths.

**Workflow shell:** Optional on feature tiers. Use it when S5D needs to support a delivery/discovery process with explicit phases, roles, review policy, and outcome tracking. The workflow shell does not replace the core S5D reasoning model.

**Ralph runtime modes:** `s5d execute loop ... --mode init|bugfix` specializes the emitted task package without expanding the persisted spec schema. Generated task packages are saved under `.s5d/tasks/`.

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

The canonical flow is defined in SKILL.md: `Route → Shape → Discover → Target → Decide → Spec → Run → Verify → Ship → Learn`. The gate system below operates within the Spec, Run, and Verify steps.

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
