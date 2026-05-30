# ddd-refactor — pattern catalog

**last_verified: 2026-05-30**

ddd-refactor's checks are self-contained grep heuristics (no install). This catalog is
the DDD reference behind each check + the tooling that helps execute a `validate`.

## Strategic DDD

| Pattern | What it buys | Check it backs |
|---|---|---|
| Bounded Context | one model per context, explicit boundaries | ubiquitous-language-drift |
| Context Map | name the relationships (partnership, customer-supplier, conformist, ACL) | missing-acl |
| Anti-Corruption Layer | translate a vendor/legacy model so it doesn't leak into the domain | missing-acl |
| Ubiquitous Language | one term → one meaning → one type, per context | ubiquitous-language-drift |

## Tactical DDD

| Pattern | What it buys | Check it backs |
|---|---|---|
| Entity | identity + lifecycle | anemic-domain |
| Value Object | immutable, equality-by-value, enforces format/units (Money, Email) | value-objects-as-primitives |
| Aggregate + Root | a consistency boundary; outside code touches only the root | aggregate-boundary-leak |
| Repository | persistence behind an interface, per aggregate | repository-absence |
| Domain Service | logic that isn't a natural fit on one entity | domain-logic-in-controllers |
| Application Service / Use Case | orchestration; thin controllers delegate to it | transaction-script |
| Domain Event | record something happened, decouple side effects | (detect: events) |

## Canonical references

- Eric Evans, *Domain-Driven Design* (2003) — the strategic/tactical vocabulary.
- Vaughn Vernon, *Implementing Domain-Driven Design* (2013) — aggregates, ACL, events in practice.
- Vernon, *Domain-Driven Design Distilled* (2016) — the short version for sequencing.
- Martin Fowler — "AnemicDomainModel", "ValueObject", "TransactionScript" (martinfowler.com).
- Alberto Brandolini — EventStorming, for discovering aggregates/events with the team.

## Tooling for the `validate` side

- **dependency-cruiser** (JS/TS, `npm i -D dependency-cruiser`) — enforce "controllers
  must not import prisma", "only the adapter imports the vendor SDK" as CI rules. This
  turns many of this skill's `validate` greps into enforced architecture tests.
- **ts-arch / arch-unit-ts** — architecture assertions as unit tests (layer dependency direction).
- **ESLint `no-restricted-imports`** — cheap guard: forbid the vendor SDK outside its adapter dir.
- For Go: **go-arch-lint** or **import-boundaries** — same layering enforcement.

The point: once a DDD move lands, encode its `validate` as a lint/arch rule so the model
can't silently rot back. A move without an enforced boundary regresses.

## Notes

- Re-run the agentic research behind this list if `last_verified` is > ~12 months stale
  (the books are stable; the tooling versions are not).
- DDD is a direction, not a binary. A thin CRUD app does not need aggregates — apply
  judgment; the skill surfaces candidates, the human decides which contexts are core.
