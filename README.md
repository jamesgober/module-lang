<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br>
    <b>module-lang</b>
    <br>
    <sub><sup>MODULE RESOLUTION</sup></sub>
</h1>

<div align="center">
    <a href="https://crates.io/crates/module-lang"><img alt="Crates.io" src="https://img.shields.io/crates/v/module-lang"></a>
    <a href="https://crates.io/crates/module-lang"><img alt="Downloads" src="https://img.shields.io/crates/d/module-lang?color=%230099ff"></a>
    <a href="https://docs.rs/module-lang"><img alt="docs.rs" src="https://img.shields.io/docsrs/module-lang"></a>
    <a href="https://github.com/jamesgober/module-lang/actions"><img alt="CI" src="https://github.com/jamesgober/module-lang/actions/workflows/ci.yml/badge.svg"></a>
    <a href="https://github.com/rust-lang/rfcs/blob/master/text/2495-min-rust-version.md"><img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.85%2B-blue"></a>
</div>

<br>

<div align="left">
    <p>
        module-lang resolves modules and imports across multiple source files. Given the modules a front-end has already parsed &mdash; each exporting a set of named items &mdash; it answers the question every <code>use</code>/<code>import</code> asks: which item does this name refer to? It builds the module graph, resolves each import to its target, reports unresolved names and import cycles as defined errors, and honours item visibility. It is a SEMA-tier crate of the <code>-lang</code> language-construction family.
    </p>
    <br>
    <hr>
    <p>
        <strong>MSRV is 1.85+</strong> (Rust 2024 edition).
    </p>
    <blockquote>
        <strong>Status: stable (1.0).</strong> The public API is frozen and follows Semantic Versioning &mdash; no breaking changes before <code>2.0</code>. See <a href="./docs/API.md#semver-promise"><code>the SemVer promise</code></a> and <a href="./CHANGELOG.md"><code>CHANGELOG.md</code></a>.
    </blockquote>
</div>

<hr>
<br>

## Installation

```toml
[dependencies]
module-lang = "1.0"
```

Or from the terminal:

```bash
cargo add module-lang
```

<br>

## Usage

Build a graph with one module per source file, define the items each exports, wire
the imports between them, and resolve a name to the payload it refers to. The
payload type is yours — a definition id, an AST node, a type — written here as
`&str` for illustration.

```rust
use intern_lang::Interner;
use module_lang::{ModuleGraph, ResolveError, Visibility};
use source_lang::SourceMap;

// Two files, each a module: `app` uses an item exported by `util`.
let mut sources = SourceMap::new();
let app_src = sources.add("app.lang", "use util.helper;")?;
let util_src = sources.add("util.lang", "pub fn helper() {}")?;

let mut names = Interner::new();
let helper = names.intern("helper");

let mut graph: ModuleGraph<&str> = ModuleGraph::new();
let app = graph.add_module(names.intern("app"), app_src);
let util = graph.add_module(names.intern("util"), util_src);

graph.define(util, helper, Visibility::Public, "fn helper")?;
graph.import(app, util, helper)?;

// The import resolves through to the definition in `util`.
assert_eq!(graph.resolve(app, helper), Ok(&"fn helper"));

// A name nobody declared is a defined error, never a panic.
let missing = names.intern("nope");
assert!(matches!(graph.resolve(app, missing), Err(ResolveError::Unresolved { .. })));
# Ok::<(), Box<dyn std::error::Error>>(())
```

Visibility gates crossing a module boundary, not seeing a module's own names. A
module reaches its own private items; an import from elsewhere does not:

```rust
use intern_lang::Interner;
use module_lang::{ModuleGraph, ResolveError, Visibility};
use source_lang::SourceMap;

let mut sources = SourceMap::new();
let mut names = Interner::new();
let mut graph: ModuleGraph<()> = ModuleGraph::new();

let lib = graph.add_module(names.intern("lib"), sources.add("lib", "")?);
let app = graph.add_module(names.intern("app"), sources.add("app", "")?);
let secret = names.intern("secret");

graph.define(lib, secret, Visibility::Private, ())?;
assert!(graph.resolve(lib, secret).is_ok());           // visible at home

graph.import(app, lib, secret)?;
assert!(matches!(                                       // not through an import
    graph.resolve(app, secret),
    Err(ResolveError::Private { .. }),
));
# Ok::<(), Box<dyn std::error::Error>>(())
```

Imports chain — a module may re-export what it imported — and a chain that loops
back is reported as a cycle in bounded time rather than recursing without end:

```rust
use intern_lang::Interner;
use module_lang::{ModuleGraph, ResolveError, Visibility};
use source_lang::SourceMap;

let mut sources = SourceMap::new();
let mut names = Interner::new();
let mut graph: ModuleGraph<()> = ModuleGraph::new();

let a = graph.add_module(names.intern("a"), sources.add("a", "")?);
let b = graph.add_module(names.intern("b"), sources.add("b", "")?);
let x = names.intern("x");

// a imports x from b, b imports x from a: a closed loop with no definition.
graph.import(a, b, x)?;
graph.import(b, a, x)?;
assert!(matches!(graph.resolve(a, x), Err(ResolveError::ImportCycle { .. })));
# Ok::<(), Box<dyn std::error::Error>>(())
```

See <a href="./docs/API.md"><code>docs/API.md</code></a> for the full reference.

<br>

## How it works

A `ModuleGraph` holds one module per source file, each carrying the `SourceId` it
was read from and a flat namespace of names. A name is declared at most once per
module — a second definition or a colliding import is a defined error. Names live
in a `BTreeMap` keyed on the interned `Symbol`, so a lookup compares integers in
`O(log items)` and iteration is deterministic.

Resolution treats a module's own names as fully visible, public or private.
Crossing a module boundary through an import is where visibility applies: an import
reaches a name in another module only if it is public. The walk along an import
chain is iterative and tracks the `(module, name)` pairs it has seen, so a loop is
reported as a cycle rather than overflowing the stack, and a local hit allocates
nothing.

<br>

## Status

<code>v1.0.0</code> is the stable release: the public API is frozen and follows
Semantic Versioning, with no breaking changes before <code>2.0</code>. The surface is
the resolution core &mdash; <code>ModuleGraph</code> with stable
<code>ModuleId</code>s, <code>define</code>/<code>import</code>/<code>resolve</code>,
visibility, and import-cycle detection, wiring <code>symbol-lang</code> for the name
key and <code>source-lang</code> for the file each module came from. Every invariant
is property-tested against a naive reference resolver and verified on Linux, macOS,
and Windows. See the
<a href="./docs/API.md#semver-promise"><code>SemVer promise</code></a>.

<hr>
<br>

## Contributing

See <a href="./dev/DIRECTIVES.md"><code>dev/DIRECTIVES.md</code></a> for engineering standards and the definition of done. Before a PR: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` must be clean.

<br>

<div id="license">
    <h2>License</h2>
    <p>Licensed under either of</p>
    <ul>
        <li><b>Apache License, Version 2.0</b> &mdash; <a href="./LICENSE-APACHE">LICENSE-APACHE</a></li>
        <li><b>MIT License</b> &mdash; <a href="./LICENSE-MIT">LICENSE-MIT</a></li>
    </ul>
    <p>at your option.</p>
</div>

<div align="center">
  <h2></h2>
  <sup>COPYRIGHT <small>&copy;</small> 2026 <strong>James Gober <me@jamesgober.com>.</strong></sup>
</div>
