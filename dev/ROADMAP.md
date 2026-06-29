# module-lang - Roadmap

> Path from scaffold to a stable 1.0. Hard parts are front-loaded; each phase has hard exit criteria.
> Master plan: ../../_strategy/LANG_COLLECTION.md
>
> **Anti-deferral rule:** no listed hard task moves to a later phase unless this file records the move and the reason.

## v0.1.0 - Scaffold (DONE)
Compiles, CI green, structure correct, no domain logic.
- [x] Manifest, README, CHANGELOG, REPS, dual license, CI, deny, clippy, rustfmt.

## v0.2.0 - Core (THE HARD PART, NOT DEFERRED) (DONE)
Module and import resolution across multiple source files. `ModuleGraph<T>` with
stable `ModuleId`s; `define`/`import`/`resolve`; visibility and import-cycle
detection. Dependencies wired where first used: `symbol-lang` for the `Symbol`
name key, `source-lang` for the `SourceId` each module is read from.
Exit criteria:
- [x] Every public item has rustdoc + a runnable example.
- [x] Core invariants property-tested against a naive reference resolver (DIRECTIVES + API authored at this stage).

## v1.0.0 - API freeze (DONE)
Public surface stable and frozen until 2.0. No new public API — the v0.2.0 surface
is ratified as the 1.0 contract; the freeze adds only a hardened lint profile, a
serde round-trip test, a runnable example, and the stability docs.
- [x] docs/API.md marked stable; SemVer promise recorded.
- [x] Full test + benchmark suite green on all three platforms.

Deferred to a later 1.x minor (each additive, non-breaking): renamed (`as`) imports
and nested module paths.
