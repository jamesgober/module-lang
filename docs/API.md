# module-lang &mdash; API Reference

> Complete reference for every public item in `module-lang`, with examples.
> **Status: stable (1.0).** The surface below is the `1.0` contract; it follows
> [Semantic Versioning](#semver-promise) and will not change in a breaking way
> before `2.0`. See [`../dev/ROADMAP.md`](../dev/ROADMAP.md).

## Table of Contents

- [Overview](#overview)
- [SemVer promise](#semver-promise)
- [Installation](#installation)
- [Quick start](#quick-start)
- [The model](#the-model)
- [`ModuleGraph`](#modulegraph)
  - [`ModuleGraph::new`](#modulegraphnew)
  - [`ModuleGraph::with_capacity`](#modulegraphwith_capacity)
  - [`ModuleGraph::add_module`](#modulegraphadd_module)
  - [`ModuleGraph::define`](#modulegraphdefine)
  - [`ModuleGraph::import`](#modulegraphimport)
  - [`ModuleGraph::resolve`](#modulegraphresolve)
  - [`ModuleGraph::len` / `is_empty`](#modulegraphlen--is_empty)
  - [`ModuleGraph::module_name` / `module_source`](#modulegraphmodule_name--module_source)
- [`Visibility`](#visibility)
- [`ModuleId`](#moduleid)
- [`ResolveError`](#resolveerror)
- [Re-exported types](#re-exported-types)
- [Feature flags](#feature-flags)

---

## Overview

module-lang resolves modules and imports across multiple source files. A
[`ModuleGraph`](#modulegraph) holds one module per source file; each module
declares names — by [`define`](#modulegraphdefine) for items it owns, or by
[`import`](#modulegraphimport) for items it pulls in from another module — and
[`resolve`](#modulegraphresolve) answers which definition a name refers to.

It sits in the SEMA tier of the `-lang` family, above
[`symbol-lang`](https://docs.rs/symbol-lang), which interns the names this crate
keys on, and [`source-lang`](https://docs.rs/source-lang), which owns the files the
modules came from. It owns module and import resolution only — no parsing, no type
checking, no code generation.

---

## SemVer promise

As of `1.0.0` the public API documented here is **stable**. The crate follows
[Semantic Versioning](https://semver.org):

- No item in this reference will be **removed or changed in a breaking way** within
  the `1.x` series. Breaking changes wait for `2.0`.
- New functionality arrives in **minor** releases (`1.1`, `1.2`, …) and is additive.
  [`ResolveError`](#resolveerror) is `#[non_exhaustive]`, so a new error variant is a
  minor change, not a breaking one — match it with a wildcard arm. Renamed (`as`)
  imports and nested module paths, if they land, arrive the same way.
- Bug fixes, documentation, and internal optimisation are **patch** releases.
- The **MSRV** is Rust `1.85`. Raising it is treated as a minor change and called
  out in the changelog; it is never a patch.

Anything not in this reference — internal modules, field layouts, exact `Debug`
output — is not part of the contract and may change at any time.

---

## Installation

```toml
[dependencies]
module-lang = "1.0"
```

The examples below also use `intern-lang` (to mint names) and `source-lang` (to
mint the source ids each module records); a consumer of module-lang already depends
on both, since their handle types appear in this crate's signatures:

```toml
intern-lang = "1"
source-lang = "1"
```

MSRV: Rust 1.85 (Rust 2024 edition).

---

## Quick start

```rust
use intern_lang::Interner;
use module_lang::{ModuleGraph, ResolveError, Visibility};
use source_lang::SourceMap;

let mut sources = SourceMap::new();
let mut names = Interner::new();
let helper = names.intern("helper");

let mut graph: ModuleGraph<&str> = ModuleGraph::new();
let app = graph.add_module(names.intern("app"), sources.add("app", "").expect("fits"));
let util = graph.add_module(names.intern("util"), sources.add("util", "").expect("fits"));

graph.define(util, helper, Visibility::Public, "fn helper").expect("unique");
graph.import(app, util, helper).expect("unique");

assert_eq!(graph.resolve(app, helper), Ok(&"fn helper"));

let missing = names.intern("nope");
assert!(matches!(graph.resolve(app, missing), Err(ResolveError::Unresolved { .. })));
```

---

## The model

Modules are added in order and identified by a stable [`ModuleId`](#moduleid). Each
module's names live in a `BTreeMap` keyed on the interned [`Symbol`](#re-exported-types),
so a lookup compares integers in `O(log items)` and iteration is deterministic. A
name is declared at most once per module — a second definition or a colliding
import is a [`ResolveError::DuplicateName`](#resolveerror).

A module's own names are fully visible from within it, public or private. Crossing
a module boundary through an import is where [`Visibility`](#visibility) applies: an
import reaches a name in another module only if it is `Public`. Imports chain (a
module may re-export what it imported); the resolver walks the chain iteratively and
records the `(module, name)` pairs it has seen, so a loop is a
[`ResolveError::ImportCycle`](#resolveerror) in bounded time, never unbounded
recursion.

---

## `ModuleGraph`

```rust
pub struct ModuleGraph<T> { /* private */ }
```

The owner of module and import resolution state, generic over the payload `T` a name
resolves to. `T` is whatever the language wants — a definition id, an AST node
handle, a type — and carries no trait bounds. Implements `Default` (= `new`), and
`Debug` / `Clone` when `T` does.

### `ModuleGraph::new`

```rust
pub const fn new() -> Self
```

Creates an empty graph.

```rust
use module_lang::ModuleGraph;

let graph: ModuleGraph<()> = ModuleGraph::new();
assert!(graph.is_empty());
```

### `ModuleGraph::with_capacity`

```rust
pub fn with_capacity(modules: usize) -> Self
```

Creates an empty graph with room for `modules` modules before reallocating.

**Parameters**

- `modules` — the number of modules to pre-allocate space for. A hint only; the
  graph still grows as needed. Useful when the file count is known up front, to add
  every module without an intermediate reallocation.

```rust
use module_lang::ModuleGraph;

let graph: ModuleGraph<()> = ModuleGraph::with_capacity(64);
assert!(graph.is_empty());
```

### `ModuleGraph::add_module`

```rust
pub fn add_module(&mut self, name: Symbol, source: SourceId) -> ModuleId
```

Adds a module and returns its stable [`ModuleId`](#moduleid).

**Parameters**

- `name` — the module's display name, for diagnostics. Does not participate in
  resolution, which is always by id.
- `source` — the [`SourceId`](#re-exported-types) of the file the module was read
  from. Recorded for diagnostics; retrievable with
  [`module_source`](#modulegraphmodule_name--module_source).

The returned id is the module's insertion order and never changes — hold it to
define names in the module, import from it, or resolve against it.

```rust
use intern_lang::Interner;
use module_lang::ModuleGraph;
use source_lang::SourceMap;

let mut sources = SourceMap::new();
let mut names = Interner::new();
let mut graph: ModuleGraph<()> = ModuleGraph::new();

let a = graph.add_module(names.intern("a"), sources.add("a.lang", "").expect("fits"));
let b = graph.add_module(names.intern("b"), sources.add("b.lang", "").expect("fits"));

assert_eq!(a.to_u32(), 0);
assert_eq!(b.to_u32(), 1);
assert_eq!(graph.len(), 2);
```

### `ModuleGraph::define`

```rust
pub fn define(
    &mut self,
    module: ModuleId,
    name: Symbol,
    visibility: Visibility,
    value: T,
) -> Result<(), ResolveError>
```

Defines `name` in `module` with a visibility and a payload.

**Parameters**

- `module` — the module to define the name in.
- `name` — the name being defined.
- `visibility` — [`Public`](#visibility) to allow other modules to import it,
  [`Private`](#visibility) to keep it reachable only from within `module`.
- `value` — the payload the name resolves to.

**Errors**

- [`DuplicateName`](#resolveerror) if `module` already declares `name` (by an
  earlier definition or an import).
- [`UnknownModule`](#resolveerror) if `module` was not minted by this graph.

```rust
use intern_lang::Interner;
use module_lang::{ModuleGraph, ResolveError, Visibility};
use source_lang::SourceMap;

let mut sources = SourceMap::new();
let mut names = Interner::new();
let mut graph: ModuleGraph<i32> = ModuleGraph::new();

let m = graph.add_module(names.intern("m"), sources.add("m", "").expect("fits"));
let x = names.intern("x");

graph.define(m, x, Visibility::Public, 42).expect("first definition is unique");
assert_eq!(graph.resolve(m, x), Ok(&42));

// A second definition of the same name in the same module is rejected.
assert!(matches!(
    graph.define(m, x, Visibility::Private, 7),
    Err(ResolveError::DuplicateName { .. }),
));
```

### `ModuleGraph::import`

```rust
pub fn import(
    &mut self,
    into: ModuleId,
    from: ModuleId,
    name: Symbol,
) -> Result<(), ResolveError>
```

Imports `name` into `into` from the `from` module, keeping its spelling. This
records an import edge; it does not require `name` to be defined in `from` yet, so a
graph can be built in any order. Whether the import actually reaches a public
definition is determined when the name is [`resolve`](#modulegraphresolve)d.

**Parameters**

- `into` — the module the name is imported into.
- `from` — the module the name is imported from.
- `name` — the name to import. The same spelling is used in both modules (renamed
  imports are not part of this release).

**Errors**

- [`DuplicateName`](#resolveerror) if `into` already declares `name`.
- [`UnknownModule`](#resolveerror) if either id was not minted by this graph.

```rust
use intern_lang::Interner;
use module_lang::{ModuleGraph, Visibility};
use source_lang::SourceMap;

let mut sources = SourceMap::new();
let mut names = Interner::new();
let mut graph: ModuleGraph<()> = ModuleGraph::new();

let lib = graph.add_module(names.intern("lib"), sources.add("lib", "").expect("fits"));
let app = graph.add_module(names.intern("app"), sources.add("app", "").expect("fits"));
let item = names.intern("item");

graph.define(lib, item, Visibility::Public, ()).expect("unique");
graph.import(app, lib, item).expect("unique");
assert!(graph.resolve(app, item).is_ok());
```

A module may re-export what it imported, and a chain of re-exports resolves to the
origin:

```rust
# use intern_lang::Interner;
# use module_lang::{ModuleGraph, Visibility};
# use source_lang::SourceMap;
# let mut sources = SourceMap::new();
# let mut names = Interner::new();
let mut graph: ModuleGraph<i32> = ModuleGraph::new();
let c = graph.add_module(names.intern("c"), sources.add("c", "").expect("fits"));
let b = graph.add_module(names.intern("b"), sources.add("b", "").expect("fits"));
let a = graph.add_module(names.intern("a"), sources.add("a", "").expect("fits"));
let item = names.intern("item");

graph.define(c, item, Visibility::Public, 123).expect("unique");
graph.import(b, c, item).expect("unique"); // b re-exports c::item
graph.import(a, b, item).expect("unique"); // a imports through b
assert_eq!(graph.resolve(a, item), Ok(&123));
```

### `ModuleGraph::resolve`

```rust
pub fn resolve(&self, module: ModuleId, name: Symbol) -> Result<&T, ResolveError>
```

Resolves `name` as seen from within `module`, returning the payload it refers to. A
name defined in the module wins, public or private. A name imported into the module
is followed to its source module — and onward along any chain of re-imports — to the
public definition it names.

**Parameters**

- `module` — the module the name is looked up in.
- `name` — the name to resolve.

**Errors**

- [`Unresolved`](#resolveerror) — the name is declared nowhere on the path.
- [`Private`](#resolveerror) — an import reached a name private to another module.
- [`ImportCycle`](#resolveerror) — the import chain loops.
- [`UnknownModule`](#resolveerror) — `module` is not from this graph.

```rust
use intern_lang::Interner;
use module_lang::{ModuleGraph, ResolveError, Visibility};
use source_lang::SourceMap;

let mut sources = SourceMap::new();
let mut names = Interner::new();
let mut graph: ModuleGraph<&str> = ModuleGraph::new();

let m = graph.add_module(names.intern("m"), sources.add("m", "").expect("fits"));
let here = names.intern("here");
let gone = names.intern("gone");

graph.define(m, here, Visibility::Public, "local").expect("unique");
assert_eq!(graph.resolve(m, here), Ok(&"local"));
assert!(matches!(graph.resolve(m, gone), Err(ResolveError::Unresolved { .. })));
```

### `ModuleGraph::len` / `is_empty`

```rust
pub fn len(&self) -> usize
pub fn is_empty(&self) -> bool
```

The number of modules in the graph, and whether it holds none.

```rust
use intern_lang::Interner;
use module_lang::ModuleGraph;
use source_lang::SourceMap;

let mut sources = SourceMap::new();
let mut names = Interner::new();
let mut graph: ModuleGraph<()> = ModuleGraph::new();
assert!(graph.is_empty());

graph.add_module(names.intern("m"), sources.add("m", "").expect("fits"));
assert_eq!(graph.len(), 1);
assert!(!graph.is_empty());
```

### `ModuleGraph::module_name` / `module_source`

```rust
pub fn module_name(&self, module: ModuleId) -> Option<Symbol>
pub fn module_source(&self, module: ModuleId) -> Option<SourceId>
```

The display name and the [`SourceId`](#re-exported-types) a module was added with,
or `None` if `module` is not from this graph. Useful for turning a
[`ResolveError`](#resolveerror)'s ids back into something a diagnostic can render.

```rust
use intern_lang::Interner;
use module_lang::ModuleGraph;
use source_lang::SourceMap;

let mut sources = SourceMap::new();
let mut names = Interner::new();
let mut graph: ModuleGraph<()> = ModuleGraph::new();

let name = names.intern("m");
let src = sources.add("m.lang", "").expect("fits");
let m = graph.add_module(name, src);

assert_eq!(graph.module_name(m), Some(name));
assert_eq!(graph.module_source(m), Some(src));
```

---

## `Visibility`

```rust
pub enum Visibility {
    Public,
    Private,
}
```

Whether an item is visible outside the module that declares it. Visibility gates
*cross-module* access only: a module always reaches its own names, public or
private, but an import from another module reaches a name only if it is `Public`. A
`Private` name reached through an import resolves to
[`ResolveError::Private`](#resolveerror). Derives `Clone`, `Copy`, `Debug`,
`PartialEq`, `Eq`, `Hash`, and (under the `serde` feature) `Serialize` /
`Deserialize`.

```rust
use intern_lang::Interner;
use module_lang::{ModuleGraph, ResolveError, Visibility};
use source_lang::SourceMap;

let mut sources = SourceMap::new();
let mut names = Interner::new();
let mut graph: ModuleGraph<()> = ModuleGraph::new();

let lib = graph.add_module(names.intern("lib"), sources.add("lib", "").expect("fits"));
let app = graph.add_module(names.intern("app"), sources.add("app", "").expect("fits"));
let secret = names.intern("secret");

graph.define(lib, secret, Visibility::Private, ()).expect("unique");
assert!(graph.resolve(lib, secret).is_ok());            // visible at home

graph.import(app, lib, secret).expect("unique");
assert!(matches!(                                        // not through an import
    graph.resolve(app, secret),
    Err(ResolveError::Private { .. }),
));
```

---

## `ModuleId`

```rust
pub struct ModuleId(/* private */);

pub const fn to_u32(self) -> u32
```

A small, copyable handle to one module in a graph — a 32-bit index minted by
[`add_module`](#modulegraphadd_module), stable for the life of the graph. The id is
opaque (no public constructor), so it can only come from the graph that holds the
module it names; an id from a different graph is a
[`ResolveError::UnknownModule`](#resolveerror), never a panic. `to_u32` returns the
raw index — useful as a dense key for a side table of per-module data. Derives
`Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, and
(under `serde`) `Serialize` / `Deserialize`.

```rust
use intern_lang::Interner;
use module_lang::ModuleGraph;
use source_lang::SourceMap;

let mut sources = SourceMap::new();
let mut names = Interner::new();
let mut graph: ModuleGraph<()> = ModuleGraph::new();

let m = graph.add_module(names.intern("m"), sources.add("m", "").expect("fits"));
assert_eq!(m.to_u32(), 0);
```

---

## `ResolveError`

```rust
#[non_exhaustive]
pub enum ResolveError {
    UnknownModule(ModuleId),
    DuplicateName { module: ModuleId, name: Symbol },
    Unresolved { module: ModuleId, name: Symbol },
    Private { module: ModuleId, name: Symbol },
    ImportCycle { module: ModuleId, name: Symbol },
}
```

The reason a graph operation or a name resolution did not succeed. Each variant
carries the [`ModuleId`](#moduleid) and [`Symbol`](#re-exported-types) it concerns,
so a caller can recover the human-readable spelling from its own interner and build
a richer diagnostic. Implements `Display` (a terse fallback naming the raw ids,
since module-lang does not own the interner) and `std::error::Error`. Derives
`Clone`, `Debug`, `PartialEq`, `Eq`.

| Variant | Raised by | Meaning |
|---------|-----------|---------|
| `UnknownModule(id)` | any method taking a `ModuleId` | the id was not minted by this graph |
| `DuplicateName { module, name }` | `define`, `import` | the name is already declared in the module |
| `Unresolved { module, name }` | `resolve` | the name is declared nowhere on the path |
| `Private { module, name }` | `resolve` | an import reached a name private to its module |
| `ImportCycle { module, name }` | `resolve` | the import chain loops back on itself |

The enum is `#[non_exhaustive]`: a downstream `match` must include a wildcard arm, so
later additions never force a breaking change on callers.

```rust
use intern_lang::Interner;
use module_lang::{ModuleGraph, ResolveError, Visibility};
use source_lang::SourceMap;

let mut sources = SourceMap::new();
let mut names = Interner::new();
let mut graph: ModuleGraph<()> = ModuleGraph::new();

let a = graph.add_module(names.intern("a"), sources.add("a", "").expect("fits"));
let b = graph.add_module(names.intern("b"), sources.add("b", "").expect("fits"));
let x = names.intern("x");

// An import chain that loops back is reported, not followed forever.
graph.import(a, b, x).expect("unique");
graph.import(b, a, x).expect("unique");
match graph.resolve(a, x) {
    Err(ResolveError::ImportCycle { module, name }) => {
        // `module_name` turns the id back into something printable.
        assert_eq!(graph.module_name(module), Some(names.intern("a")));
        let _ = name;
    }
    other => panic!("expected a cycle, got {other:?}"),
}
```

---

## Re-exported types

The foreign handle types this crate's API speaks in are re-exported, so a consumer
need not also name `symbol-lang` / `source-lang` just to call it:

- `Symbol` — from [`symbol-lang`](https://docs.rs/symbol-lang) (originally
  [`intern-lang`](https://docs.rs/intern-lang)); the interned name key every module
  and item is named by. Mint one with an `intern_lang::Interner`.
- `SourceId` — from [`source-lang`](https://docs.rs/source-lang); the stable id of
  the file a module was read from. Mint one by adding a source to a
  `source_lang::SourceMap`.

---

## Feature flags

| Feature | Default | Description |
| ------- | ------- | ----------- |
| `std`   | yes     | Standard-library support, forwarded to `source-lang` and `symbol-lang`. With it disabled the crate is `#![no_std]` and resolution runs on `alloc` alone. |
| `serde` | no      | Derives `serde::Serialize` / `Deserialize` for the plain handle and metadata types — [`ModuleId`](#moduleid) and [`Visibility`](#visibility) — for tooling that stores resolution references. Serialising a whole `ModuleGraph` is not offered here: its keys are `Symbol`s, whose serialisation belongs to the interner that owns the strings. |

Features are additive: enabling one never removes or changes behaviour provided by
another, per the project's SemVer policy.

<sub>Copyright &copy; 2026 <strong>James Gober</strong>.</sub>
