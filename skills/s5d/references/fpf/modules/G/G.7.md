---
id: "G.7"
title: "Cross‑Tradition Bridge Calibration Kit (BridgeMatrix → BridgeCards + BCT/Sentinels)"
kind: "pattern"
part: "G"
status: "Stable"
source:
  path: "source/FPF-Spec.md"
  sha256: "cb4666c3c2f5482e59d6d79f39e48e2a6706d40e243c6617a348ba4a0823c371"
  start_line: 65484
  end_line: 65870
relations:
  builds_on:
    - "G.2"
    - "F.9"
    - "F.3"
    - "F.7"
    - "B.3"
    - "G.6"
    - "E.18"
    - "A.21"
    - "E.10"
    - "C.21"
  prerequisite_for:
    - "G.5"
    - "G.11"
    - "G.9"
    - "G.10"
    - "G.12"
  used_by:
    - "G.9"
    - "G.11"
    - "G.10"
    - "G.12"
  uses:
    - "G.2"
    - "F.9"
    - "C.21"
    - "C.18"
    - "C.19"
    - "C.23"
    - "G.6"
    - "G.4"
---

## G.7 - Cross‑Tradition Bridge Calibration Kit (BridgeMatrix → BridgeCards + BCT/Sentinels)

**Tag.** Architectural pattern
**Stage.** design‑time (calibration + publication) + run‑time (sentinel‑driven telemetry emission; orchestration owned by **G.11**)
**Primary output.** A bridge calibration kit that turns **G.2**’s BridgeMatrix rows into **F.9** `BridgeCard`s and publishes: a `BridgeCalibrationTable (BCT)` + `CalibrationLedger` + `RegressionSet` + `SentinelSet`, plus UTS‑visible crossing rows and RSCR‑ready sentinel triggers scoped to `PathSliceId` / `PatternScopeId`.
**Primary hooks.** `G.Core` (Part‑G invariants + RSCR trigger catalogue + default-ownership index), **G.2** (BridgeMatrix), **F.9** (BridgeCard + CL/CL^k), **F.3/F.7** (SenseCell anchoring; row bottleneck discipline), **E.18/A.21** (GateCrossing + CrossingBundle checks), **G.6** (PathId/PathSliceId citation surface), **G.5** (downstream consumer for eligibility/selection), **G.11** (refresh orchestration consumer), **B.3** (assurance lanes + penalty policies), **C.21** (DHC accounts such as AlignmentDensity), **C.18/C.19** (QD/OEE pins when relevant), **C.23** (SoS‑LOG clauses as explainability gates for cross‑Tradition choices), **G.4** (Acceptance hooks/thresholds when bridges are used as selector gates), **E.10** (LEX / strict distinction discipline).
**Working‑Model first.** Prefer a minimal, auditable calibration procedure and worked micro‑cases; escalate to heavier harnesses only where risk warrants (per **E.8**).
**Non‑duplication note.** Universal Part‑G invariants (no shadow specs; Bridge‑only crossings; penalty routing to `R_eff` only; P2W split; typed/id‑based RSCR causes; single‑owner defaults; Δ‑discipline) are owned by `G.Core` and are *cited* via `CC‑GCORE‑*`. This pattern defines only the *bridge calibration kit* and its surfaces.

### G.7:1 - Problem frame

SoTA synthesis (**G.2**) can legitimately preserve pluralism by exporting a **BridgeMatrix**: a Tradition×Tradition inventory of “comparable constructs” with preliminary notes (candidate correspondences, likely losses, tentative levels). Downstream patterns (CHR/CAL/selector/logging/shipping) cannot consume this safely unless cross‑Context reuse is:

* **materialised** as explicit bridge artefacts (not implied by prose),
* **calibrated** with a small, auditable procedure (so CL/CL^k/plane routing is not a narrative),
* **published** as checkable crossing bundles (UTS + GateCrossing harness),
* **refreshable** in a *targeted* way (path‑scoped RSCR rather than whole‑pack reruns).

`G.7` packages this into a kit: `BCT` + `BridgeCard` publication + `RegressionSet`/`SentinelSet` wiring, so that later patterns can satisfy core invariants without re‑inventing cross‑Tradition machinery.

### G.7:2 - Problem

1. Cross‑Tradition comparisons are frequently attempted via informal “synonymy” or ad‑hoc mappings, causing silent meaning drift and hidden crossings.
2. Plane mismatches (world ↔ concept ↔ episteme, or other `ReferencePlane` shifts) are often ignored, or conflated with “semantic sameness”, causing wrong downstream confidence.
3. Calibration changes (CL/CL^k/plane or their policy pins) must trigger **targeted** re‑checks; pack‑wide reweaves are too costly and too slow.
4. If bridges are involved in QD/illumination or other edition‑sensitive telemetry, **edition pins** must be tracked (otherwise comparisons become irreproducible after a map/distance/policy update).
5. Row‑level summaries (for matrix rows / comparable construct groups) tend to be averaged or “smoothed”, which is incompatible with bottleneck semantics and loss honesty.

### G.7:3 - Forces

| Force                                    | Tension                                                                                                                                                                   |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Comparability vs local authority**     | Enable comparisons across Traditions ↔ avoid overriding Context‑local meaning.                                                                                            |
| **Auditability vs authoring throughput** | Require explicit artefacts, losses, and pins ↔ keep the calibration procedure light enough to be used.                                                                    |
| **Targeted refresh vs safety**           | Emit path‑local RSCR triggers ↔ ensure triggers are typed and carry enough payload pins for audit and rerun planning.                                                     |
| **Plane awareness vs “one story”**       | Explicitly surface `ReferencePlane` and plane penalties ↔ avoid turning plane discussion into a second semantics of “sameness”.                                           |
| **QD comparability vs metric drift**     | Enable cross‑context reporting of archive/illumination telemetry ↔ enforce edition‑aware pins for descriptor/distance/policies only when those modes are actually in use. |

### G.7:4 - Solution — Bridge calibration kit (BCT + BridgeCards + RegressionSet/Sentinels)

#### G.7:4.1 - G.Core linkage (normative)

**Builds on:** `G.Core` (Part‑G core invariants; routing/delegation hub)

**GCoreLinkageManifest (normative).**

`GCoreLinkageManifest := ⟨  
  CoreConformanceProfileIds := {  
    GCoreConformanceProfileId.PartG.AuthoringBase,  
    GCoreConformanceProfileId.PartG.TriStateGuard,  
    GCoreConformanceProfileId.PartG.UTSWhenPublicIdsMinted  
  },  
  RSCRTriggerSetIds := { GCoreTriggerSetId.BridgeCalibrationKit },  
  CorePinSetIds := { GCorePinSetId.PartG.CrossingVisibilityPins },  
  CorePinsRequired := {  
    BridgeCalibrationTableId (BCT.id),  
    RegressionSetId,  
    SentinelSetId,  
    FreshnessWindowRef,  
    CalibrationLedgerId,  
    RowScopeId,  
    ReferencePlane(src),  
    ReferencePlane(tgt),  
    UTSRowId[],  
    PathId[]/PathSliceId[]  
  },  
  DefaultsConsumed := ∅,  
  TriggerAliasMapRef := ∅  
⟩`

* **Expansion rule.** Effective `CoreConformanceIds`, `RSCRTriggerKindIds`, and `CorePinsRequired` are obtained by expanding the cited profile/set ids and unioning with the explicit ids above (see `G.Core` nil‑elision + expansion rule).
* **Conditional pins.**
  * `BridgeCardRef.edition` is required iff BridgeCards are published as editioned artefacts.
  * Sentinel scopes MAY be recorded as `PatternScopeId[]` when path surfaces are not available (and SHALL then be present in sentinel records and emitted trigger payload pins).
* **CN/CG note.** `CC‑GCORE‑CN‑CG‑1` is included via `GCoreConformanceProfileId.PartG.AuthoringBase` and is exercised only when the governance card and legality gate (e.g., `CNSpecRef.edition` / `CGSpecRef.edition`) are explicitly pinned; penalty/guard policy ids (`Φ(CL)`, `Ψ(CL^k)`, `Φ_plane`) are policy pins, not governance cards or legality gates.

*(payload pins, minimum: affected members of the effective `CorePinsRequired` (after expansion) plus any pins introduced by active extensions (e.g., QD parity pins), scoped to the watched `PathSliceId[]`/`PathId[]`/`PatternScopeId[]`.)*

#### G.7:4.2 - Kit objects (pattern‑owned surfaces)

This pattern defines the *bridge calibration kit* as a set of minimal, checkable surfaces. Semantics of `BridgeCard` and CL typing are owned by **F.9**; G.7 adds calibration artefacts and publication/wiring surfaces.

**(A) BridgeCalibrationTable (BCT) — object.**
A `BridgeCalibrationTable` is a per‑Tradition‑pair registry of calibrated bridge entries.

Minimal fields (conceptual):

`BridgeCalibrationTable := ⟨  
BCT.id, TradPairId, FreshnessWindowRef,  
RowEntries[]  
⟩`

**Source provenance (when sourced from `G.2`).** If the BCT is derived from a `G.2` BridgeMatrix, publish `BridgeMatrixId` (+ `BridgeMatrixRef.edition` when editioned) and row‑level linkage via `G.7:Ext.MatrixIntake` (wiring‑only), rather than duplicating G.2 semantics in core.

Where each `RowEntry` minimally binds:

`RowEntry := ⟨  
RowEntryId, ComparableConstructId, RowScopeId,  
BridgeCardId[],  
RowCL_min, RowCL_k_min?, RowCL_plane_min?,  
LossNoteRef[]?, CounterExampleRef[]?, CounterExampleAbsenceRef?, WaiverRef[]?,  
RegressionSetId, SentinelSetId,  
PolicyPins: { Φ(CL), Ψ(CL^k)?, Φ_plane? },  
PlanePins: { ReferencePlane(src), ReferencePlane(tgt) },  
ExtensionPins?: { [GPatternExtensionId]: { …ids… } }  
⟩`

**(B) CalibrationLedger — object.**
A `CalibrationLedger` is the auditable “row narrative” that remains *pin‑first*: it records what was calibrated, what was lost, and which artefacts/policies witness that.

Minimal fields:

`CalibrationLedger := ⟨  
LedgerId, TradPairId,  
Entries[]  // each entry cites RowEntryId, BridgeCardId(s), CL‑minima, waivers (if any), loss notes, counterexamples, UTS rows, and (when run) regression-run/delta refs  
⟩`

**(C) RegressionSet — object.**
A `RegressionSet` is a small set of regression probes/checks that are runnable against the BCT row entries. It exists to detect drift (bridge edits, policy edits, plane edits, edition pin changes) and to provide the evidential payload for RSCR triggers.

Minimal fields:

`RegressionSet := ⟨ RegressionSetId, TradPairId, TestCaseId[], ExpectedOutcomesRef?, RegressionRunRef? ⟩`

##### G.7:4.2.1 - CL / CL^k admissibility regime and plane guard (kit‑local; normative)

This subsection is kit‑owned (G.7) and complements (but does not duplicate) `G.Core` penalty routing and tri‑state guard semantics.

**Admissibility regimes (row‑level, minimal).**
* `RowCL_min` MUST take a value in `{3,2,1,0}` (value set and CL meaning are owned by F.9; G.7 owns the admissibility regime).
* Default admissibility for cross‑Tradition reuse:
  * `RowCL_min ≥ 2` ⇒ admissible for reuse (subject to downstream guards/policies).
  * `RowCL_min = 1` ⇒ **NOT** admissible unless an explicit `WaiverRef[]` is cited; any reuse under waiver is **guarded-only** (no substitution semantics).
  * `RowCL_min = 0` ⇒ forbidden for reuse; it MAY remain in BCT as a documented non‑bridge with loss notes/counterexamples.
* **Honesty rule (row‑level):**
* if `RowCL_min ≤ 2`, at least one `CounterExampleRef[]` MUST be cited;
* if `RowCL_min = 3` and `CounterExampleRef[]` is empty, a citable `CounterExampleAbsenceRef` MUST be provided (explicit “searched‑none found / no known counterexample” disclosure);
  * if any `LossNoteRef[]` is present, the row MUST NOT be presented as “free substitution” in any consumer surface.

**Kind channel (`CL^k`) (conditional).**
If a row relies on bridges in the `Kind` channel, then `RowCL_k_min` and `Ψ(CL^k)` pin MUST be present, and the same admissibility regimes apply to `RowCL_k_min`.

**Plane guard (`CL^plane`) (conditional).**
If `ReferencePlane(src)` and `ReferencePlane(tgt)` differ (or plane routing is explicitly invoked), then:
* `RowCL_plane_min` and `Φ_plane` pin MUST be present;
* if either plane pin is absent, the row is non‑conformant (no implicit plane defaulting);
* any “blocking” outcome must be representable downstream via `G.Core` tri‑state guard (`abstain` or a policy‑bound `degrade(mode=…)`), without introducing additional statuses in G.7;
* plane effects MUST NOT rewrite `CL/CL^k`; their impact is routed via the pinned policy ids and `G.Core` penalty semantics.

**(D) SentinelSet & BridgeSentinel — object.**
A `SentinelSet` is a watch‑list that connects bridge calibration changes to RSCR‑ready triggers scoped to downstream consumption.

Minimal fields:

`BridgeSentinel := ⟨  
SentinelId,  
watchedBridgeIds: BridgeCardId[],  
watchedScope: PathSliceId[] | PathId[] | PatternScopeId[],  
payloadPins: { BCT.id, RegressionSetId, FreshnessWindowRef, PolicyPins, PlanePins, UTSRowId[] }  
⟩`

`SentinelSet := ⟨ SentinelSetId, BridgeSentinel[] ⟩`

#### G.7:4.3 - Minimal calibration procedure (auditable; table‑backed; bridge‑first)

For each Tradition‑pair and each comparable construct row from **G.2**:

1. **Materialise bridge artefacts.** Produce (or reuse) **F.9** `BridgeCard`s for the concrete `SenseCell`‑level alignments required by the row scope.
   *Note.* “SenseCell anchoring” is a kit requirement: if a row is authored at a coarser token level, the SenseCell anchors must be explicitly cited (F.3 discipline).
2. **Record row scope and losses.** Author a `RowScopeId` and record loss notes as first‑class citations (e.g., `LossNoteRef[]`), not as informal footnotes.
   Also record `RowCL_min` (and `RowCL_k_min?`, `RowCL_plane_min?` when applicable) and cite `WaiverRef[]` if any row is intentionally kept at `=1` for guarded-only reuse.
3. **Plane pins (no hidden plane mixing).** Record `ReferencePlane` pins for source/target and the relevant policy id pins for plane routing (ids only; do not duplicate policy tables).
4. **Policy pins for penalty routing.** Record the policy id pins needed to audit penalty routing (ids only). Penalty semantics are core‑owned (route via `CC‑GCORE‑PEN‑1`); G.7’s responsibility is to make the pins explicit and published.
5. **Row bottleneck discipline.** When a row aggregates multiple bridge cells, row summarisation uses bottleneck semantics (F.7) and carries a counterexample citation whenever any cell is loss‑noted.
6. **Regression and sentinel wiring.** Create/update the `RegressionSet` and `SentinelSet`. Any calibration change that can affect downstream audit (CL/CL^k/plane pins, relevant policy ids, edition pins for involved telemetry surfaces, freshness window) emits typed RSCR triggers (canonical ids; scope + payload pins).
   If the regression harness is run, record a citable `RegressionRunRef` (or equivalent run/delta reference) and attach it to the relevant ledger entries (pin‑first; no narrative-only deltas).

#### G.7:4.4 - Publication surfaces (UTS + GateCrossing harness)

A conformant G.7 publication:

* publishes UTS‑citable identifiers for `BridgeCard`s and any GateCrossing/crossing rows that rely on them,
* ensures crossing bundles are checkable via **E.18/A.21** harnesses (lexical SD, lane purity, required pin presence),
* emits RSCR triggers using canonical `RSCRTriggerKindId` and attaches the minimum payload pins listed in §4.1.
* ensures evidence-facing citations are pin-complete: whenever bridge calibration is cited in SCR/Evidence surfaces, the citation MUST include `{BCT.id, RegressionSetId}` and the active policy id pins `{Φ(CL), Ψ(CL^k)?, Φ_plane?}` (ids only; representation is owned by `G.6`/SCR).

#### G.7:4.5 - Worked mini‑examples (informative; post‑2015; row scopes + loss notes)

> These are **working models**, not equivalence claims. They illustrate how row scope + loss notes constrain safe reuse.

1. **Preference‑learning objective (Method; RowScope = “training‑objective‑intent”).**
   *Cells:* `RLHF@Context‑A` ↔ `DPO@Context‑B` ↔ `IPO@Context‑C`
   *RowCL_min:* 2 (guarded)
   *Loss notes:* different inductive biases (reward model vs direct preference likelihood; sensitivity to preference noise model; implicit regularisation forms).
   *Use:* cross‑Tradition *didactic alignment* and eligibility hints; thresholds/acceptance remain CAL‑owned.

2. **Robustness evaluation (Measurement; RowScope = “metric‑family‑intent”).**
   *Cells:* `Accuracy@IID` ↔ `Robustness@ShiftBench` (e.g., distribution‑shift benchmarks common in post‑2019 practice)
   *RowCL_min:* 2
   *Loss notes:* shift taxonomy differs; comparability depends on pinned protocol editions and window selection; “robustness” is not a scalar substitute for accuracy.

3. **Quality‑Diversity archive comparability (Measurement; RowScope = “DescriptorMap‑only”).**
   *Cells:* `MAP‑Elites grid indices` ↔ `CVT‑MAP‑Elites centroids` ↔ `CMA‑ME archive`
   *RowCL_min:* 2
   *Loss notes:* discretisation vs centroidal tessellation; archive pressure differs; drift occurs if `DistanceDef` or insertion policy changes.
   *Use:* lawful cross‑reporting of QD telemetry when edition pins are explicit.

4. **Open‑ended transfer semantics (Method; RowScope = “transfer‑rule intent”).**
   *Cells:* `POET‑class transfer rule` ↔ `Enhanced‑POET‑class transfer rule` ↔ “modern open‑ended transfer variants”
   *RowCL_min:* 2
   *Loss notes:* environment validity region differs; transfer timing and selection pressures differ; pinning transfer rule editions is mandatory for audit.

#### G.7:4.6 - Extensions (pattern‑scoped; non‑core)

> Extensions carry *wiring only* (pins/editions/policy‑ids + which semantic owners are used). They MUST NOT redefine core invariants or defaults.

**GPatternExtension: MatrixIntake**

* **PatternScopeId:** `G.7:Ext.MatrixIntake`
* **GPatternExtensionId:** `MatrixIntake`
* **GPatternExtensionKind:** `InteropSpecific`
* **SemanticOwnerPatternId:** `G.2` *(BridgeMatrix semantics and comparable-construct inventory)*
* **Uses:** `{G.2, F.9}`
* **⊑/⊑⁺:** `∅`
* **RequiredPins/EditionPins/PolicyPins (minimum):**

  * `BridgeMatrixId` (and, if editioned: `BridgeMatrixRef.edition`)
  * `BridgeMatrixRowRef[]` *(row‑level anchors for intake; owner‑defined; e.g., `PatternScopeId` / `UTSRowId` / row ids)*
  * `ComparableConstructId[]` *(row keys; if the source does not supply a stable id, `G.7` mints one while preserving `BridgeMatrixRowRef` as the provenance anchor)*
  * `LossNoteRef[]?` *(if exported by `G.2`; otherwise authored in `G.7` and cited from the `CalibrationLedger`)*
* **RSCRTriggerKindIds:** `{RSCRTriggerKindId.CrossingBundleEdit, RSCRTriggerKindId.EvidenceSurfaceEdit, RSCRTriggerKindId.EditionPinChange}`
* **Notes (wiring‑only):** This module binds “row candidates” from G.2 to the BCT/Ledger intake without copying G.2 semantics into G.7.

**GPatternExtension: DHCAccounting**

* **PatternScopeId:** `G.7:Ext.DHCAccounting`
* **GPatternExtensionId:** `DHCAccounting`
* **GPatternExtensionKind:** `DisciplineSpecific`
* **SemanticOwnerPatternId:** `C.21` *(DHC metric semantics, including AlignmentDensity)*
* **Uses:** `{C.21}`
* **⊑/⊑⁺:** `∅`
* **RequiredPins/EditionPins/PolicyPins (minimum; conditional on use):**

  * `AlignmentDensityMethodRef.edition?`
  * `DeclaredUnitsRef?` *(units declaration style per owner; e.g., “bridges_per_100_DHC_SenseCells”)*
* **RSCRTriggerKindIds:** `{RSCRTriggerKindId.TelemetryDelta, RSCRTriggerKindId.PolicyPinChange, RSCRTriggerKindId.EditionPinChange}`
* **Notes (wiring‑only):**
  * G.7 stores the *counts and declared units* as a surface; C.21 owns the meaning and legality constraints.
  * When reporting AlignmentDensity, the counted bridge set is typically restricted to `CL ≥ 2` (treat `CL=3` as “free substitution”, `CL=2` as “guarded” for reporting); conformance is enforced by `CC‑G7‑DHC‑Units‑1` while semantics remain owned by `C.21`.

**GPatternExtension: QDParityPins**

* **PatternScopeId:** `G.7:Ext.QDParityPins`
* **GPatternExtensionId:** `QDParityPins`
* **GPatternExtensionKind:** `InteropSpecific`
* **SemanticOwnerPatternId:** `C.18` *(QD artefact semantics; uses C.19 for exploration/logging pins as needed)*
* **Uses:** `{C.18, C.19}`
* **⊑/⊑⁺:** `∅`
* **RequiredPins/EditionPins/PolicyPins (minimum; conditional on use):**

  * `DescriptorMapRef.edition`
  * `DistanceDefRef.edition`
  * `InsertionPolicyRef` *(policy id or pinned policy ref, per owner semantics)*
* **RSCRTriggerKindIds:** `{RSCRTriggerKindId.EditionPinChange, RSCRTriggerKindId.PolicyPinChange, RSCRTriggerKindId.TelemetryDelta, RSCRTriggerKindId.FreshnessOrDecayEvent}`
* **Notes (wiring‑only):** Enforces reproducibility of cross‑Context archive/illumination comparisons without pulling QD semantics into the core bridge kit.
  The pins from this module should be attached via `RowEntry.ExtensionPins[QDParityPins]` (or an equivalent extension‑pin map) and included in `BridgeSentinel.payloadPins` whenever the watched scope consumes QD telemetry.

**GPatternExtension: SoSLogClauses**

* **PatternScopeId:** `G.7:Ext.SoSLogClauses`
* **GPatternExtensionId:** `SoSLogClauses`
* **GPatternExtensionKind:** `InteropSpecific`
* **SemanticOwnerPatternId:** `C.23` *(SoS‑LOG rule and branch semantics; G.7 does not redefine meaning)*
* **Uses:** `{C.23, G.6}`
* **⊑/⊑⁺:** `∅`
* **RequiredPins/EditionPins/PolicyPins (minimum; conditional on use):**
  * `SoSLogRuleId[]` *(or owner‑equivalent ids)*
  * `FailureBehaviorPolicyId?` *(policy id, when degrade behavior is bound)*
  * `PathId/PathSliceId` citations for explainability (via `G.6`)
  * `BridgeCardId[]` (bridges whose reuse is being justified)
* **RSCRTriggerKindIds:** `{RSCRTriggerKindId.PolicyPinChange, RSCRTriggerKindId.EvidenceSurfaceEdit, RSCRTriggerKindId.CrossingBundleEdit, RSCRTriggerKindId.MaturityRungChange}`
* **Notes (wiring‑only):** Ensures cross‑Tradition bridge reuse decisions can be justified by citing SoS‑LOG clauses and evidence paths, without embedding SoS‑LOG semantics into G.7.

**GPatternExtension: AcceptanceHooks**

* **PatternScopeId:** `G.7:Ext.AcceptanceHooks`
* **GPatternExtensionId:** `AcceptanceHooks`
* **GPatternExtensionKind:** `MethodSpecific`
* **SemanticOwnerPatternId:** `G.4` *(Acceptance/threshold/unknown handling; G.7 does not define thresholds)*
* **Uses:** `{G.4}`
* **⊑/⊑⁺:** `∅`
* **RequiredPins/EditionPins/PolicyPins (minimum; conditional on use):**
  * `AcceptanceClauseId[]` *(or owner‑equivalent ids)*
  * `AcceptancePolicyId?` *(policy id when acceptance behavior is pinned)*
  * `BridgeCardId[]` (bridges whose calibrated status is being used as a gate input)
* **RSCRTriggerKindIds:** `{RSCRTriggerKindId.PolicyPinChange, RSCRTriggerKindId.BaselineBindingEdit, RSCRTriggerKindId.LegalitySurfaceEdit}`
* **Notes (wiring‑only):** When bridges are used as selector gates, thresholds and unknown-handling remain Acceptance-owned; this module only pins the linkage and refresh relevance.

**GPatternExtension: AdvancedCalibrationProcedures (Phase‑3 seed)**

* **PatternScopeId:** `G.7:Ext.AdvancedCalibrationProcedures`
* **GPatternExtensionId:** `AdvancedCalibrationProcedures`
* **GPatternExtensionKind:** `Phase3Seed`
* **SemanticOwnerPatternId:** `owner TBD`
* **Uses:** `{ }`
* **⊑/⊑⁺:** `∅`
* **RequiredPins/EditionPins/PolicyPins:** `owner TBD`
* **RSCRTriggerKindIds:** `{RSCRTriggerKindId.CrossingBundleEdit, RSCRTriggerKindId.PenaltyPolicyEdit, RSCRTriggerKindId.ReferencePlaneEdit}`
* **Notes (seed; non‑normative):** Placeholder for domain‑specific / statistical calibration families beyond the minimal auditable procedure (e.g., uncertainty‑aware calibration, probabilistic mapping). No Part‑G‑wide norms are introduced.

### G.7:5 - Archetypal Grounding (System / Episteme)

**System (Γ_sys):** *Cross‑standard safety assurance comparison (bridge‑first).*
A team must compare a safety assurance claim across two regulatory Traditions (e.g., a “functional safety case” tradition and a “ML system testing” tradition) for the *same physical system scope*. `G.7` forces explicit SenseCell‑level bridges (what exactly is the “hazard”, what is the “evidence carrier”, what is the “pass criterion”), records losses, pins planes, and provides sentinels so that changes in the safety evidence protocol editions trigger path‑local RSCR rather than re‑authoring the entire safety case.

**Episteme (Γ_epist):** *Benchmark protocol pluralism (post‑2015 evaluation practice).*
A research group wants to compare “state‑of‑the‑art” across multiple evaluation Traditions (IID performance, shift robustness, preference‑based evaluation). `G.7` turns “these are comparable” into explicit BridgeCards with declared row scope, pins the evaluation protocol editions, and emits sentinels so that when a benchmark protocol or policy pin changes, downstream selector decisions can be re‑audited by re‑citing the same PathSlice‑scoped evidence.

### G.7:6 - Bias‑Annotation

Lenses tested: **Gov**, **Arch**, **Onto/Epist**, **Prag**, **Did**.
Scope: Universal for the bridge calibration kit; any method‑family or discipline‑specific calibration technique is modularized as `GPatternExtension` and cited to its semantic owners.

### G.7:7 - Conformance Checklist (normative) — **CC‑G7**

| ConformanceId             | Requirement                                                                                                                                                                                                                                                                               | Purpose                                                                        |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| **CC‑G7‑CoreRef**         | `G.7` is conformant only if it satisfies the effective `G.Core` obligations declared by the `GCoreLinkageManifest` in **§4.1** (after nil‑elision and expansion of profile/set/pinset ids), including any explicit deltas listed there. | Make universal invariants single‑owner and enforce citation‑based reuse.       |
| **CC‑G7‑BCT‑1**           | For any active `TradPairId` with cross‑Tradition reuse, a `BridgeCalibrationTable (BCT)` **MUST** exist, declare a `FreshnessWindowRef`, and provide `RowEntry` records that cite, at minimum: `RowEntryId`, `ComparableConstructId`, `RowScopeId`, `BridgeCardId[]`, `RowCL_min`, `PlanePins {ReferencePlane(src), ReferencePlane(tgt)}`, `PolicyPins {Φ(CL)}` (and `Ψ(CL^k)?`, `Φ_plane?` when applicable), plus `{RegressionSetId, SentinelSetId}`. | Ensure the kit exists as an auditable object rather than a prose matrix.       |
| **CC‑G7‑BridgeCard‑1**    | Any bridge published by G.7 **MUST** be consumable as an **F.9** `BridgeCard` and **MUST** be SenseCell‑anchored (directly or via explicit SenseCell anchor refs).                                                                                                                        | Prevent “Context‑only” or ambiguous bridges.                                   |
| **CC‑G7‑UTS‑1**           | G.7 outputs **MUST** mint/publish UTS‑citable ids (NameCards/twin labels as applicable) for (a) each BridgeCard (or its NameCard) and (b) each GateCrossing/crossing row that makes bridge use checkable; and **MUST** expose the resulting `UTSRowId[]` in the BCT/Ledger/crossing bundles. *(UTS discipline is delegated to `CC‑GCORE‑UTS‑1`.)* | Make bridge calibration externally citable and checkable.                      |
| **CC‑G7‑RowScope‑1**      | Every BCT row **MUST** declare its `RowScopeId` (what notion of “sameness” is claimed), and any loss notes **MUST** be recorded as citable artefacts (refs/ids), not only narrative text.                                                                                                 | Keep reuse honest and locally bounded.                                         |
| **CC‑G7‑CLRegime‑1**      | Every BCT row **MUST** record `RowCL_min` (and `RowCL_k_min?`, `RowCL_plane_min?` where applicable) and apply the admissibility regime from §4.2.1: `≥2` admissible; `=1` only with cited `WaiverRef[]`; `=0` forbidden for reuse. The honesty rule must be satisfied: ≥1 counterexample for `≤2`, and an explicit stated‑absence disclosure for `=3` when no counterexample is cited. | Make CL/waiver/plane regimes explicit and auditable at kit level.              |
| **CC‑G7‑SCRLinkage‑1**    | Whenever bridge calibration is cited in SCR/Evidence surfaces, the citation **MUST** include `{BridgeCardId[]}` (or `UTSRowId[]` for the bridge artefacts), an explicit row locator (`RowEntryId` or equivalent), `{BCT.id, RegressionSetId}`, and the active policy id pins `{Φ(CL), Ψ(CL^k)?, Φ_plane?}` (ids only; representation owned elsewhere). | Prevent “pins exist but are not visible/auditable” failure mode.               |
| **CC‑G7‑SoSLOG‑Pins‑1**   | When `G.7:Ext.SoSLogClauses` is in use, G.7 outputs **MUST** expose the cited SoS‑LOG rule ids and the relevant `PathId/PathSliceId` evidence citations; any change in those pins **MUST** be RSCR‑relevant per `CC‑GCORE‑TRIG‑1…TRIG‑4`.                                               | Keep cross‑Tradition reuse explainable without embedding C.23 semantics.        |
| **CC‑G7‑Acceptance‑1**    | When `G.7:Ext.AcceptanceHooks` is in use, G.7 outputs **MUST** expose the Acceptance clause ids/policy ids used as gates; thresholds/unknown handling remain Acceptance-owned; any change **MUST** be RSCR‑relevant per `CC‑GCORE‑TRIG‑1…TRIG‑4`.                                           | Keep thresholds and unknowns out of bridges while preserving auditability.     |
| **CC‑G7‑RowBottleneck‑1** | If a comparable construct row aggregates multiple bridge cells, row summaries (e.g., `RowCL_min`) **MUST** follow bottleneck discipline (F.7) and cite a counterexample whenever a cell carries a loss note.                                                                              | Forbid “CL averaging” and enforce loss‑aware summaries.                        |
| **CC‑G7‑PolicyPins‑1**    | G.7 outputs **MUST** publish the *policy id pins* required to audit penalty routing and plane effects (ids only), as required by `CC‑GCORE‑LINK‑1/2` and `CC‑GCORE‑PEN‑1`. G.7 MUST NOT duplicate policy tables or redefine penalty semantics.                                           | Keep penalty routing auditable while preserving single‑owner policy semantics. |
| **CC‑G7‑GateCrossing‑1**  | Any published crossing rows that rely on bridges **MUST** be checkable via GateCrossing/CrossingBundle harnesses (E.18/A.21): required pins are present; lexical constraints and lane purity checks are runnable.                                                                        | Make crossings checkable, not narrative.                                       |
| **CC‑G7‑Sentinels‑1**     | G.7 **MUST** register `BridgeSentinel` entries for bridges used by live scopes and **MUST** emit typed RSCR triggers (canonical `RSCRTriggerKindId`; see `CC‑GCORE‑TRIG‑1…TRIG‑4`) on calibration‑relevant edits, scoped to the watched `PathSliceId[]` or `PatternScopeId[]`, with the minimum payload pins from §4.1. | Enable targeted refresh rather than pack‑wide reruns.                          |
| **CC‑G7‑QD‑Pins‑1**       | When `G.7:Ext.QDParityPins` is in use, G.7 outputs **MUST** include `{DescriptorMapRef.edition, DistanceDefRef.edition, InsertionPolicyRef}` and treat any change to those pins as RSCR‑relevant per `CC‑GCORE‑TRIG‑1…TRIG‑4`.                                                          | Prevent silent QD telemetry drift.                                             |
| **CC‑G7‑DHC‑Units‑1**     | When AlignmentDensity (or related DHC accounts) are reported, G.7 outputs **MUST** (a) restrict the counted bridge set to rows with `RowCL_min ≥ 2` (treat `CL=3` as “free substitution”, `CL=2` as “guarded” for reporting), (b) include declared units, and (c) cite the relevant DHC method semantics (C.21). G.7 MUST NOT invent arithmetic over ordinal/illegal surfaces. | Keep dashboards and discipline‑health metrics lawful and interpretable.        |

### G.7:8 - Common Anti-Patterns and How to Avoid Them

* **Bridge‑by‑prose (“they mean the same thing”).**
  **Avoid:** publish BCT rows + BridgeCards + UTS rows; require SenseCell anchoring and row scopes.
* **SenseFamily jump (scope‑bridge used as kind‑bridge).**
  **Avoid:** keep channel/sense‑family constraints owned by **F.9** visible; use `RowScopeId` to state which channel is claimed, and require `CL^k` + `Ψ(CL^k)` pins when a kind‑channel bridge is invoked (do not “upgrade” a scope‑channel bridge into kind substitution).
* **Plane blindness (“concept = world”).**
  **Avoid:** record plane pins and policy id pins; keep plane effects auditable and separable from CL/CL^k semantics.
* **CL smoothing / averaging.**
  **Avoid:** enforce row bottleneck summaries and counterexample citations for loss‑noted cells.
* **Pack‑wide refresh on a local bridge edit.**
  **Avoid:** register sentinels scoped to `PathSliceId` and emit typed RSCR triggers with minimal payload pins.
* **QD metric drift by unpinned artefacts.**
  **Avoid:** enable `G.7:Ext.QDParityPins` only when needed and require edition/policy pins when enabled.

### G.7:9 - Consequences

* **Auditable pluralism.** Cross‑Tradition reuse becomes explicit, loss‑aware, and checkable.
* **Targeted, edition‑aware refresh.** Calibration drift triggers path‑scoped RSCR rather than expensive global reruns.
* **Downstream cleanliness.** Selectors/logging/shipping can cite bridges and policy pins without inventing local crossing rules or shadow specs.

### G.7:10 - Rationale

* **Why a kit (not a new governance card or legality gate)?** Bridge calibration must support many downstream consumers without becoming a competing legality gate; contract semantics remain owned by `CG‑Spec`/`CN‑Spec`.
* **Why BCT + RegressionSet + SentinelSet?** Because calibration without regression tests drifts silently, and regression without sentinels is operationally unusable (refresh becomes global).
* **Why row scopes?** Because “comparable” is not one thing; scope must be explicit to avoid accidental substitution.

### G.7:11 - SoTA‑Echoing (post‑2015, for orientation; non‑normative)

* **Edition‑aware evaluation and dataset shift practice.** Post‑2018 evaluation culture (robustness and shift benchmarks, protocol pinning, reproducibility checklists) motivates treating protocol versions and “what changed” as first‑class pins rather than prose.
* **Preference‑based optimisation families.** Modern preference‑learning lines (late‑2010s → 2020s) show how neighbouring objectives can share intent but diverge in assumptions—an archetypal case for row scope + loss notes.
* **Quality‑Diversity and differentiable QD.** MAP‑Elites successors (CVT variants, CMA‑ME line, differentiable QD ecosystems) emphasise archive/descriptor/distance artefacts whose editions must be pinned for comparability.
* **Open‑ended evolution and transfer‑rule portfolios.** POET‑class work motivates explicit transfer rule editions and environment validity regions as pins when bridges are used for cross‑tradition reporting.

### G.7:12 - Relations

**Builds on:** `G.Core`, `G.2`, `F.3`, `F.7`, `F.9`, `B.3`, `E.10`, `E.18`, `A.21`, `G.6`, `C.21`.
**Optionally uses via Extensions:** **G.4** (Acceptance hooks), **C.23** (SoS‑LOG clauses), **C.18/C.19** (QD/OEE pins).
**Used by / prerequisite for:** **G.5** (cross‑Tradition eligibility/selection), **G.11** (refresh orchestration), **G.9** (parity across Traditions where bridges are required), **G.10** (shipping surfaces that must cite bridge calibration ids), **G.12** (DHC dashboards when bridge counts/units are surfaced).
**Publishes to:** **UTS** (bridge and crossing rows; twin labels as applicable) and emits RSCR‑ready telemetry/trigger payloads for **G.11**.
**Constrains:** Any downstream consumer that claims cross‑Context/Tradition reuse must use the calibrated bridge artefacts/pins surfaced by this kit (core‑owned crossing invariants apply).

### G.7:End

