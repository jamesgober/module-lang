# module-lang &mdash; Engineering Directives

> Engineering standards and the definition of done for this project. Read alongside `REPS.md` (root, authoritative) and `dev/ROADMAP.md` (current phase). If anything here conflicts with `REPS.md`, `REPS.md` wins.

---

## 0. Philosophy

This library is built and maintained to a production standard and treated as a flagship piece of work. Plan the full path, then build one verified step at a time. "Good enough" is treated as a defect. module-lang is the layer a multi-file front-end resolves names through: every import a program writes is matched here to the item it names, so a wrong answer is a miscompile, not a cosmetic bug. The public surface stays small on purpose — resolution is the one job, and it is done exactly.

---

## 1. What this is

module-lang resolves modules and imports across multiple source files. It takes the modules a front-end has already parsed — each exporting a set of named items — and answers the question every `use`/`import` asks: *which item does this name refer to?* It builds the module graph, resolves each import to its target, reports unresolved names and import cycles as defined errors, and honours item visibility. It sits in the SEMA tier above `symbol-lang`, which interns the names, and `source-lang`, which owns the files the modules came from. It owns module and import resolution only — no parsing, no type checking, no code generation.

---

## 2. Engineering law (non-negotiable)

- **Performance** — peak is the baseline; a name resolves through a module's exports in `O(log items)`, never a linear scan of every item; the graph is built once and queried many times, not re-walked per lookup; no steady-state hot-path allocation on a local-resolution hit; no "faster" claim without `criterion` numbers.
- **Correctness** — the invariants in section 4 are covered by property tests; a resolved import names exactly the item a hand-traced resolution would, and an import cycle terminates as a reported error, never as unbounded recursion.
- **Security** — every module and import is treated as untrusted input; a malformed graph, a self-import, or a cycle is a defined error, never UB or a stack overflow; recursion depth and fan-out are bounded.
- **Architecture** — SOLID, KISS, YAGNI; one responsibility; the dependencies (`symbol-lang`, `source-lang`) sit behind narrow seams and are wired only where first used.
- **Cross-platform** — Linux/macOS/Windows first-class, verified by CI; nothing here is platform-specific, and it stays that way.
- **Error handling** — every fallible path (unknown module, unresolved import, duplicate definition, import cycle) returns `Result` per the documented contract; no panics in shipping code.
- **Production-ready** — `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]` from the first commit; no stray `println!`/`dbg!`; every public item has rustdoc with a runnable example.

---

## 3. Definition of done

1. Compiles clean on Linux/macOS/Windows, stable and MSRV 1.85.
2. `fmt`, `clippy -D warnings`, `test --all-features`, `cargo doc -D warnings` clean.
3. `cargo audit` + `cargo deny check` pass.
4. No `unwrap`/`expect`/`todo!`/`dbg!` in shipping code.
5. A Tier-1 API exists and headlines the docs.
6. Property tests cover every section-4 invariant.
7. Hot-path changes carry benchmarks; no regression over 5%.
8. Docs and `CHANGELOG.md` updated; the matching `docs/release/vX.Y.Z.md` written before the tag.

---

## 4. Project-specific invariants

- Every module added receives a unique, stable `ModuleId` that never changes for the life of the graph.
- Resolving an import yields exactly the item the name refers to under the visibility rules, or a defined "unresolved" error — never the wrong item and never a panic.
- A private item is never resolvable from outside the module that declares it.
- An import cycle is detected and reported as a defined error in bounded time; resolution never recurses without limit.
- A duplicate definition of the same name within one module is a defined error, surfaced where it is added — not silently last-write-wins.
- Resolution is deterministic: the same set of modules and imports always produces the same result, independent of insertion order beyond what the rules specify.
