# Tribunal round 1 — Codex adversarial review (2026-06-12)

Scope: PR #13 diff (branch voroninr/ran-491-ownership-derivation vs main).
Reviewer ran: `git diff --check`, `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --all` — all passed.

## VERDICT: REJECT (both blockers confirmed by maintainer review and fixed in round 2)

### Blocking 1 — derivation not anchored to imported content
`derive_global_owners` applied CURRENT package files to HISTORICAL ledger entries,
ignoring `LedgerEntry.spec_sha256`. A backdated package edit could fabricate a
false owner mismatch and veto a legitimate rollback.

**Fix:** per-entry sha gating. An import/reconcile entry contributes claims only
when the current file's sha256 matches the entry. A mismatching/missing file
poisons every key unowned at that replay point (→ `Unverifiable`), cleared only
by that package's own rollback entry. Unverifiable never routes to the
destructive fallback — a two-file tamper (alias field + any package file) must
not be able to downgrade the veto into a tombstone. Pinned by
`rollback_skips_tombstoning_when_ownership_unverifiable`.

### Blocking 2 — rollback epochs ignored
`AliasTable::resolve` is first-creator-wins **within the active epoch** (rollback
tombstones the entry; the next import creates a fresh one with a new owner). The
original replay derived "first ever importer" and would false-flag the legitimate
second-epoch owner on an untampered repo (A imports X → A rolled back → B imports
X → B rollback vetoed).

**Fix:** the replay releases a package's ownerships (and its poison entries) when
its ledger `rollback` entry is seen. Pinned by
`rollback_after_reimport_does_not_false_flag_new_owner`.

### Non-blocking notes (accepted)
- Fingerprint untouched — confirmed.
- Tombstoning still gated by stored claim — derivation never expands destruction.
- CLI/MCP mutation order centralized, matches old shape.
- Remaining test-gap notes (MCP warning parity, reconcile-derived ownership)
  acknowledged; MCP shares the lib function and report serialization verbatim.

---

# Tribunal round 2 — Codex (2026-06-12)

## VERDICT: REJECT (both lifecycle blockers confirmed and fixed in round 3)

### Blocking 1 — self-poisoning on normal re-import
Edit → re-approve → re-import leaves the FIRST ledger entry with a stale sha;
that self-stale entry poisoned the package's own keys and degraded its own
rollback to "unverifiable" skip.

**Fix:** self-poison is harmless by construction — whichever of a package's
imports was first, the owner is the same package. Read-rule: a verified
Owner stands unless the poison set contains a FOREIGN package. Pinned by
`rollback_after_edit_and_reimport_tombstones_own_globals`.

### Blocking 2 — rolled-back spec file pins globals via referenced-guard
`referenced_globals` walked all package files regardless of record status, so
a rolled-back spec left on disk kept its globals alive forever (pre-existing
behavior, now inconsistent with epoch semantics).

**Fix:** specs whose record status is Deprecated no longer count as
referencing. Pinned by
`rollback_epoch_transition_works_with_old_spec_file_left_on_disk`.

Also introduced `DerivedOwnership::UnverifiedCandidate` — a key whose only
ledger candidate is unverified maps to fallback-tombstone when consistent
with the stored field, and to non-destructive skip otherwise.
