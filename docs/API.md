# module-lang &mdash; API Reference

> **Status: pre-1.0 scaffold (v0.1.0).** This release stands up the crate, the
> tooling, and the quality gates; it exports **no public API yet**. The surface
> sketched below is the intended shape, not an implemented contract — each item is
> marked *planned* and lands across the 0.x series before the freeze at `1.0`.
> See [`../dev/ROADMAP.md`](../dev/ROADMAP.md) for the phase plan and
> [`../dev/DIRECTIVES.md`](../dev/DIRECTIVES.md) for the invariants it will hold to.

## Table of Contents

- [Overview](#overview)
- [Installation](#installation)
- [Feature flags](#feature-flags)
- [Planned surface](#planned-surface)
  - [`ModuleGraph` (planned)](#modulegraph-planned)
  - [`ModuleId` (planned)](#moduleid-planned)
  - [`ResolveError` (planned)](#resolveerror-planned)
- [Stability & SemVer](#stability--semver)

---

## Overview

module-lang resolves modules and imports across multiple source files. Given the
modules a front-end has already parsed — each exporting a set of named items — it
answers the question every `use`/`import` asks: *which item does this name refer
to?* It builds the module graph, resolves each import to its target, reports
unresolved names and import cycles as defined errors, and honours item visibility.

It sits in the SEMA tier of the `-lang` language-construction family, above
[`symbol-lang`](https://docs.rs/symbol-lang) (which interns the names) and
[`source-lang`](https://docs.rs/source-lang) (which owns the files the modules
came from). It owns module and import resolution only — no parsing, no type
checking, no code generation. Those dependencies are wired at v0.2.0, where the
resolution core first uses them, so the scaffold carries no unused dependency.

---

## Installation

```toml
[dependencies]
module-lang = "0.1"
```

MSRV: Rust 1.85 (Rust 2024 edition).

---

## Feature flags

| Feature | Default | Description |
| ------- | ------- | ----------- |
| `std`   | yes     | Standard-library support. With it disabled the crate is `#![no_std]`; the resolution core is being designed to hold on `alloc` alone. |
| `serde` | no      | Derives `serde::Serialize` / `Deserialize` for the graph metadata (ids and edges), for tooling that wants to inspect or cache a resolved graph. |

Features are additive: enabling one never removes or changes behaviour provided by
another, per the project's SemVer policy.

---

## Planned surface

> Nothing in this section is exported in v0.1.0. It records the intended shape so
> the public contract is legible before it is implemented, and so reviewers can
> hold the implementation to it. Names and signatures may still change before they
> first ship in a tagged release; once a symbol appears in a release it follows the
> [SemVer policy](#stability--semver).

### `ModuleGraph` (planned)

The owner of the resolution state: a set of modules, each with a stable id and a
table of exported names, plus the import edges between them. Construction is
infallible; adding modules and imports is fallible (a duplicate definition or an
unknown target is a defined error). Lookups borrow from the graph rather than
copying, and name resolution within a module is average `O(1)`.

### `ModuleId` (planned)

A small, copyable handle to a module in the graph — stable for the life of the
graph and cheap to pass around. Resolution returns ids, not borrows, so callers
hold references across further mutation.

### `ResolveError` (planned)

The single error type for the resolution surface, with one variant per defined
failure: unresolved import, duplicate definition, visibility violation, and import
cycle. Every variant carries enough context to point a diagnostic at the offending
name; none of them is a panic.

---

## Stability & SemVer

The crate follows [Semantic Versioning](https://semver.org). During the 0.x series
the public surface is still being designed, so minor releases may make breaking
changes — each is documented in [`../CHANGELOG.md`](../CHANGELOG.md) with a
migration note. At `1.0.0` the surface freezes: no breaking change before `2.0`,
additions arrive in minor releases, and the MSRV only rises in a minor release.
This file is updated in lockstep with every release so it always matches the code.

<sub>Copyright &copy; 2026 <strong>James Gober</strong>.</sub>
