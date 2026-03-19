# S5D v2 Workflow Specification

Status: Proposed  
Issue: `RAN-195`  
Scope: workflow and artifact model only, no runtime behavior changes in this document

## 1. Problem

S5D is already strong at architectural reasoning and decision discipline, but weaker as an end-to-end operating workflow for implementation.

Current gaps:
- decision quality is stronger than delivery choreography
- target architecture and delivery strategy are too easy to blur
- roles are not first-class, so multi-agent staffing is improvised too late
- long-running implementation needs a bounded autonomous loop model
- external review is available, but not yet placed explicitly in the lifecycle

This specification defines `S5D v2` as a workflow layer over the existing S5D/FPF canon.

## 2. Canonical Constraints

The following remain non-negotiable:
- `FPF` remains the reasoning baseline
- `WLNK`, `F-G-R`, `CL`, and `ADI` remain mandatory vocabulary
- S5D remains a software-architecture profile for an existing repository and codebase
- human confirmation remains mandatory for irreversible decisions
- resources may affect decisions only when explicitly declared as constraints or optimization targets
- target architecture and delivery strategy must remain separate objects

This spec does not replace S5D with OpenSpec, Spec Kit, or any other framework.

## 3. Goals

- Add an explicit `Research -> Plan -> Implement -> Operate` workflow shell around S5D
- Make roles first-class so they can drive multi-agent team formation
- Add explicit phase planning and phase acceptance
- Define where cross-model review belongs
- Define where a `Ralph`-style execution loop is allowed and where it is forbidden
- Reuse current artifacts where possible instead of inventing a parallel documentation system

## 4. Non-Goals

- Replace S5D/FPF as the canonical reasoning system
- Make S5D a universal greenfield strategy framework
- Introduce fully autonomous execution without human gates
- Import all of OpenSpec or Spec Kit wholesale
- Redesign every existing CLI command in one step

## 5. Target Workflow

`S5D v2` is the following lifecycle:

`Research -> Plan -> Implement -> Operate`

### 5.1 Research

Research is the current S5D decision path:
- Preflight
- Frame
- Hypothesize
- Evidence
- Audit
- Decide

Outputs of Research:
- framed problem
- acceptance axes
- explicit resource axes, if any
- target architecture
- initial role set
- winning hypothesis
- DRR

Research answers:
- what problem exists
- what architecture should win
- what trade-offs were accepted

Research does not answer:
- exact phase rollout
- exact staffing plan
- exact implementation sequencing

### 5.2 Plan

Plan is a new explicit layer between decision and execution.

Plan answers:
- how the chosen target architecture will be reached
- how work is divided into phases
- how each phase is accepted
- which roles are required
- which agents can be instantiated from those roles

Outputs of Plan:
- delivery strategy
- role map
- multi-agent staffing plan
- phase list
- exit criteria per phase
- rollback and escalation conditions
- review policy

### 5.3 Implement

Implementation is phase-based, not big-bang.

Execution loop:
1. activate one phase
2. instantiate agents from the recorded role map
3. execute only the active phase scope
4. run local gates
5. run review gates
6. perform human phase acceptance
7. move to the next phase

Implementation is not allowed to mutate:
- target architecture
- acceptance axes
- role model semantics

If one of those must change, execution stops and a child decision is opened.

### 5.4 Operate

Operate closes the lifecycle:
- reflect
- capture heuristics
- feed learnings into future work

## 6. Object Model

S5D v2 distinguishes the following objects:
- `Target architecture`: the desired steady-state design
- `Delivery strategy`: the migration logic used to reach that design
- `Role set`: the named responsibilities required by the work
- `Role map`: binding of responsibilities to human or agent execution
- `Phase`: a bounded implementation increment with explicit acceptance
- `Execution mode`: how a phase is run, for example `manual` or `ralph`
- `Review policy`: when cross-model review is required

These objects must not be collapsed into one another.

## 7. Artifact Model

The primary artifact remains the spec file.

`*.s5d.yaml` remains canonical for:
- problem frame
- winning architecture
- verification intent
- role map
- delivery strategy
- phases

`.record.yaml` remains mutable for:
- status progression
- approvals
- gate results
- phase acceptance history
- reflections

Optional evidence carriers may exist under `.s5d/reviews/` for external reviewer output, but the spec and record remain the system of record.

## 8. Required Additions to the Spec Schema

The target schema extension is:

```yaml
workflow:
  mode: research | plan | implement | operate
  target_architecture:
    summary: "..."
    invariants:
      - "..."
  delivery_strategy:
    summary: "..."
    rationale: "..."
  resources:
    declared: true
    constraints:
      - "token budget <= X"
      - "phase must fit in Y review window"
  role_map:
    owner: human
    researcher: planner
    planner: planner
    implementer: coder
    reviewer: reviewer
    verifier: reviewer
    operator: ops
  review_policy:
    cross_model_required: true
    required_on:
      - high_tier
      - cross_domain_change
      - protocol_change
  execution_mode:
    engine: manual | ralph
    max_iterations: 5
    stop_conditions:
      - "phase gates pass"
      - "scope exhausted"
      - "unexpected architecture change required"
  phases:
    - id: phase-1
      title: "..."
      scope: "..."
      roles:
        - implementer
        - reviewer
      acceptance:
        - "..."
      rollback:
        - "..."
```

`resources` is optional. If absent, resource optimization is not a default selection axis.

## 9. Roles and Multi-Agent Teaming

Roles are fixed during Research and refined in Plan.

Minimum role vocabulary:
- `owner`
- `researcher`
- `planner`
- `implementer`
- `reviewer`
- `verifier`
- `operator`

Agent derivation rules:
- one role may map to one human, one agent, or a human+agent pair
- one agent may hold multiple roles only if this does not violate separation of concerns
- `implementer` and `reviewer` should not default to the same agent on high-risk work
- `owner` is always human

Target agent mapping:
- `researcher` -> `planner`
- `planner` -> `planner`
- `implementer` -> `coder`
- `reviewer` -> `reviewer`
- `verifier` -> `reviewer`
- `operator` -> `ops`

## 10. Phase Model

A phase is valid only if it has:
- bounded scope
- named roles
- explicit acceptance criteria
- explicit rollback or containment

Phase acceptance is a first-class gate.

Phase transitions:
- `planned`
- `active`
- `verified`
- `accepted`
- `rolled_back`

Only one phase may be `active` at a time unless the plan explicitly marks parallel-safe phases with disjoint write scopes.

## 11. Review Model

Cross-model review is introduced as a workflow gate, not as a second decision system.

Cross-model review is mandatory when any of the following is true:
- tier is `High`
- change crosses bounded contexts
- change modifies contracts or protocols
- change alters safety-critical invariants
- human explicitly requests independent review

Cross-model review is advisory for architecture selection but blocking for execution acceptance when policy says it is required.

## 12. Ralph Loop Integration

A `Ralph`-style loop is allowed only inside an already approved phase.

It must have:
- an active phase
- a fixed scope
- explicit stop conditions
- iteration bound
- escalation rule

It must not:
- choose target architecture
- edit acceptance axes
- invent new roles
- silently expand scope

If a loop discovers that the approved plan is wrong, it stops and opens a new decision or plan revision.

## 13. External Pattern Imports

### 13.1 RPI

Imported directly as the top-level shell:
- `Research`
- `Plan`
- `Implement`

### 13.2 Cross-Model Workflow

Imported as a verification layer:
- independent reviewer model family
- especially valuable for high-blast-radius or protocol work

### 13.3 Ralph

Imported as an optional execution engine for bounded autonomous work.

### 13.4 Superpowers

Imported as an execution toolkit:
- TDD
- systematic debugging
- code review
- worktrees
- subagent-driven implementation

These are implementation techniques, not canonical reasoning primitives.

### 13.5 OpenSpec

Imported selectively:
- brownfield-first stance
- artifact-guided change organization
- explicit separation of current state and proposed change

Not imported:
- fluid replacement of S5D gates
- governance semantics that weaken human decision control

### 13.6 Spec Kit

Imported selectively:
- constitution-style governance packaging
- extension mechanism ideas
- explicit separation of spec, plan, and tasks

Not imported:
- wholesale lifecycle replacement
- a second competing canon for architecture decisions

## 14. CLI Target State

Existing commands stay valid.

New target-state commands:
- `s5d plan build <spec>`
- `s5d plan show <spec>`
- `s5d team derive <spec>`
- `s5d phase list <spec>`
- `s5d phase start <spec> --id <phase>`
- `s5d phase accept <spec> --id <phase> --reviewer <name>`
- `s5d review cross-model <spec>`
- `s5d execute loop <spec> --phase <id> --engine ralph`

This document does not require immediate implementation of all commands. It defines the target interface.

## 15. Gate Policy by Tier

### Note

No change.

### Decision

No change to human confirmation gate.

### Lightweight

Required:
- schema
- human phase acceptance

Optional:
- cross-model review

### Standard

Required:
- schema
- graph
- lint
- phase acceptance

Conditional:
- cross-model review if review policy requires it

### High

Required:
- schema
- graph
- lint
- test
- contract
- cross-model review
- human phase acceptance

## 16. Migration Strategy

Implementation should happen in this order:

1. Document the model and keep it normative
2. Add role map and delivery strategy to the spec schema
3. Add phase model to the spec and record
4. Add `team derive`
5. Add phase acceptance mechanics
6. Add cross-model review gate
7. Add optional `ralph` execution mode

This order preserves current S5D behavior while adding the new workflow shell incrementally.

## 17. Acceptance Criteria for This Spec

This specification is successful if:
- S5D remains the single canonical reasoning system
- target architecture and delivery strategy are structurally separated
- roles become explicit enough to drive multi-agent staffing
- phased implementation becomes first-class
- cross-model review gets a clear lifecycle position
- autonomous loops become bounded and subordinate to approved plans

## 18. Open Questions

- Should role and phase data live directly in the main spec schema or in a sibling workflow section file?
- Should cross-model review output be stored inline in `.record.yaml` or as an external evidence artifact referenced by the record?
- Should `ralph` execution be available only for `Standard` and `High`, or for any phased feature with explicit safeguards?
