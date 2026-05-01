# Domain-Capability Mode

S5D agent operating mode for turning product intent into an architecture-aware specification. This mode runs inside the S5D flow, mainly during Frame and Spec. It is not a standalone skill.

Goal: prevent agents from solving a feature by inventing duplicate modules, leaking entities across domains, or changing generic/supporting code because the boundary was unclear.

## Trigger

Use this mode when input is:

- raw product intent, stakeholder note, transcript, Jira/Linear ticket, or design;
- project discovery / onboarding before feature work;
- feature shaping before implementation;
- a request to produce an agent implementation brief;
- a change that may cross domain, component, UX, or contract boundaries.

## Position In S5D

Canonical path:

```text
Product Intent -> Domain-Capability Design -> S5D Spec -> Build -> Verify -> Learn
```

The mode outputs S5D-native artifacts:

- `artifacts.features`
- `artifacts.use_cases`
- `artifacts.domains`
- `artifacts.capabilities`
- `artifacts.entities`
- `artifacts.components`
- `artifacts.systems` / `containers`
- `contracts`
- `links.feature_to_domain`
- `links.use_case_to_capability`
- `links.component_to_capability`
- `links.component_to_entity`
- `links.edges`
- `gates`

## Repository Boundary

Keep discovery and feature shaping in the current repository by default. Do not introduce a separate architecture repository as the first move.

Use the existing S5D surfaces:

- `.s5d/packages/*.s5d.yaml` for approved architecture intent and feature specs;
- `.s5d/records/*.record.yaml` for runtime lifecycle state;
- existing project docs or S5D notes for temporary discovery findings that are not yet stable enough for a full spec.

Escalate to a monorepo or separate architecture repository only when the single-repo boundary is a verified bottleneck: multiple product repos need the same map, source provenance cannot be represented locally, or cross-repo drift becomes measurable.

## Init Source Survey

At project discovery init, survey available primary sources before building the map. Record the result in the S5D spec context or a local S5D note.

Ask for and classify:

- code host: GitHub or another VCS;
- tracker: Jira, Linear, or another issue system;
- design: Figma, Miro, screenshots, exports, or local design files;
- docs: README, ADRs, wiki, product notes, transcripts;
- delivery: CI, build artifacts, TestFlight/Play Console, release notes;
- runtime evidence: logs, analytics, feature flags, experiments;
- MCP connectors: available servers, resource templates, app connectors, and missing connectors.

For each source, record:

```markdown
| Source | Status | Canonical For | Access | Scope | Confidence | Notes |
|---|---|---|---|---|---|---|
| GitHub | connected | code/history | MCP/CLI | repo/branch | verified | |
| Linear | unavailable | product tasks | none | n/a | speculative | ask owner/export |
```

Statuses:

- `connected`: agent can query it now.
- `available-manual`: user can provide/export it.
- `unavailable`: known source, no access now.
- `not-used`: intentionally out of scope.

If a canonical source is unavailable, do not invent its contents. Mark the affected map entries `[SPECULATIVE]` or stop if the missing source blocks behavior, contracts, privacy/security, or irreversible scope.

## Core Distinctions

Always keep three layers separate:

- Functional dependency: domain/component uses a capability.
- Technical transport: in-memory call, SDK, HTTP, GraphQL, messaging, database read.
- Ownership: a domain owns an entity; other domains see projections/contracts.

Do not equate:

- domain with folder/crate/module;
- UX route/screen with domain;
- entity with DTO/projection;
- provider SDK with product-specific capability;
- implementation hypothesis with approved implementation.

## Current-State Discovery

Before shaping a feature, read canonical sources:

- `AGENTS.md`, `CLAUDE.md`, `README`, architecture docs;
- existing `.s5d/` specs and records;
- source modules, services, routes, schemas, models, migrations, tests;
- UX/navigation/screens/forms/route params/design files when available;
- SDKs, API clients, queues, external services, storage boundaries.

Build a current-state map:

```markdown
## Architecture Map
- Domains: id, name, classification, maturity, owner/team, confidence
- Capabilities: id, domain, name, implemented by, consumed by
- Entities: id, owning domain, lifecycle, projections, aggregate/root notes
- Use cases: name, feature/source, capabilities, entities, UX surfaces
- Components: path/module, domain, feature, container, capabilities/entities through links
- UX surfaces: screen/flow/view, bound entities, triggered capabilities, navigation inputs/outputs
- Edges: upstream domain, downstream consumer, capability, contract, transport, archetype
- Unknowns: missing evidence, why it matters, how to verify
```

Mark claims:

- `[VERIFIED]` read from code/docs/tool output.
- `[INFERRED]` deduced from verified facts.
- `[SPECULATIVE]` plausible but unconfirmed.

Never claim absence of a capability, entity, component, or dependency before searching.

## Product Intent Intake

Convert raw input into use cases before planning code.

Extract:

- role/user;
- desired outcome;
- trigger/context;
- happy path;
- important alternates/errors;
- data created/read/updated/deleted;
- UX surfaces;
- acceptance checks;
- constraints: privacy, security, latency, offline, rollout, analytics.

Ask only when missing information affects product behavior, irreversible scope, security/privacy, public contracts, cost, or external commitments. For reversible choices, choose a reasonable assumption and label it.

Output:

```markdown
## Feature Intent
<one paragraph>

## Use Cases
- <use case>: role, trigger, outcome, acceptance, unknowns

## Product Questions
- Blocking: <questions that must be answered before implementation>
- Non-blocking: <assumptions that can be revised later>
```

## Feature Impact Mapping

For each use case, trace:

```text
Feature -> UseCase -> Capability -> Entity/Projection -> Component/UX Surface -> Container/Transport
```

Declare operation per item:

- `REF`: existing item used unchanged.
- `UPDATE`: existing item changes.
- `ADD`: new item introduced.
- `DEPRECATE`: item intentionally phased out.

Prefer updating an existing capability when behavior is an evolution of the same domain contract. Add a new capability only when verb/object, owner, lifecycle, policy, or consumer contract is genuinely different.

Boundary rules:

- Generic domains must not depend on core domains.
- Supporting/generic capabilities must not learn core product details.
- Cross-domain entity access uses projections/contracts.
- UI changes bind to capabilities/entities explicitly.
- Folder/module boundaries are evidence, not proof.
- Contract changes name owner, consumer, capability, and transport.

## Implementation Hypotheses

For non-trivial work, generate at least three hypotheses before choosing. A "do nothing/manual" or clearly inferior option is acceptable when the design space is small, but label why it loses.

Each hypothesis includes:

- change summary;
- impacted domains/capabilities/entities/projections;
- impacted components/UX surfaces/containers/transports;
- contracts preserved or changed;
- expected diff radius;
- required gates/tests;
- weakest link;
- boundary risks;
- product/demo value;
- rollback or handoff cost.

Selection order:

1. Preserve domain ownership and contract direction.
2. Reuse existing capabilities, entities, and components when semantics match.
3. Minimize touched modules and cross-domain coupling.
4. Keep UX/data/navigation contracts explicit.
5. Produce the smallest working prototype that validates the product idea.
6. Record alternatives so engineering can review the decision.

Implement only the selected hypothesis unless explicitly asked to build multiple branches. Record alternatives in the spec instead.

## Scope Contract

Before coding, write the agent-operating contract:

```markdown
## Selected Hypothesis
<name and rationale>

## Allowed Changes
- <paths/components/capabilities/contracts that may change>

## Forbidden Changes
- <domains/components/contracts that must not change without approval>

## Boundary Contracts
- Owner:
- Consumer:
- Capability:
- Entity/Projection:
- Transport:

## Verification
- Architecture checks:
- Tests/typechecks/lints:
- Manual/demo checks:

## Handoff
- Product artifact:
- Spec artifact:
- Branch/build/prototype artifact:
- Known risks:
- Feedback requested from engineer:
```

Coding starts only after this contract is clear. If the user asks to "just implement" and the change is non-trivial, create the scope contract first, then proceed through S5D Build.

## Feedback Loop

Treat feedback as tuning the S5D mode:

- Feature feedback: product intent lacked use cases or acceptance checks.
- Spec feedback: hypothesis selection or boundary mapping was weak.
- Prototype feedback: implementation crossed boundaries, duplicated capability, or missed gates.
- Handoff feedback: prototype helped but required engineering repair; record what constraint would have prevented it.

## Stop Conditions

Stop and ask when:

- no repository or architecture source is available;
- product behavior is ambiguous and cannot be safely assumed;
- public API, auth, payment, PII, compliance, or irreversible data behavior changes;
- selected hypothesis requires changing a contract owned outside the current work;
- implementation would modify forbidden paths or cross declared boundaries;
- validation cannot run and no acceptable manual substitute exists.
