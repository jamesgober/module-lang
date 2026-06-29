//! # module_lang
//!
//! Module and import resolution across multiple source files. Given the modules a
//! front-end has already parsed — each exporting a set of named items — this crate
//! answers the question every `use`/`import` asks: *which item does this name refer
//! to?*
//!
//! A [`ModuleGraph`] holds one module per source file, each carrying the
//! [`SourceId`] it was read from and a flat namespace of names. A name is either
//! [`define`](ModuleGraph::define)d in the module, with a [`Visibility`] and a
//! language-supplied payload, or [`import`](ModuleGraph::import)ed into it from
//! another module. [`resolve`](ModuleGraph::resolve) walks definitions and import
//! edges to the payload a name refers to, or returns a defined [`ResolveError`] for
//! a name that is missing, private, or part of an import cycle.
//!
//! It sits in the SEMA tier of the `-lang` family, above
//! [`symbol_lang`](https://docs.rs/symbol-lang) — which interns the names this
//! crate keys on — and [`source_lang`](https://docs.rs/source-lang) — which owns
//! the files the modules came from. It owns module and import resolution only: no
//! parsing, no type checking, no code generation.
//!
//! ## Model
//!
//! Modules are added in order and identified by a stable [`ModuleId`]. Each
//! module's names live in a [`BTreeMap`](alloc::collections::BTreeMap) keyed on the
//! interned [`Symbol`], so a lookup compares integers in `O(log items)` and
//! iteration is deterministic. A name is declared at most once per module — a
//! second definition or a colliding import is a [`ResolveError::DuplicateName`].
//!
//! Resolution treats a module's own names as fully visible: a name defined in a
//! module resolves from within it whether it is public or private. Crossing a
//! module boundary through an import is where [`Visibility`] applies — an import
//! reaches a name in another module only if it is [`Visibility::Public`]. Imports
//! chain (a module may re-export what it imported), and the walk tracks the
//! `(module, name)` pairs it has seen, so a chain that loops back is reported as a
//! [`ResolveError::ImportCycle`] in bounded time rather than recursing without end.
//!
//! ## Quickstart
//!
//! ```
//! use intern_lang::Interner;
//! use module_lang::{ModuleGraph, ResolveError, Visibility};
//! use source_lang::SourceMap;
//!
//! // Two files, each a module: `app` uses an item exported by `util`.
//! let mut sources = SourceMap::new();
//! let app_src = sources.add("app.lang", "use util.helper;")?;
//! let util_src = sources.add("util.lang", "pub fn helper() {}")?;
//!
//! let mut names = Interner::new();
//! let helper = names.intern("helper");
//!
//! let mut graph: ModuleGraph<&str> = ModuleGraph::new();
//! let app = graph.add_module(names.intern("app"), app_src);
//! let util = graph.add_module(names.intern("util"), util_src);
//!
//! graph.define(util, helper, Visibility::Public, "fn helper")?;
//! graph.import(app, util, helper)?;
//!
//! // The import resolves through to the definition in `util`.
//! assert_eq!(graph.resolve(app, helper), Ok(&"fn helper"));
//!
//! // A name nobody declared is a defined error, never a panic.
//! let missing = names.intern("nope");
//! assert!(matches!(graph.resolve(app, missing), Err(ResolveError::Unresolved { .. })));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Stability
//!
//! The public API is being designed across the 0.x series and freezes at `1.0`.
//! Until then a minor release may make a breaking change, each documented in the
//! [`CHANGELOG`](https://github.com/jamesgober/module-lang/blob/main/CHANGELOG.md).
//! See [`docs/API.md`](https://github.com/jamesgober/module-lang/blob/main/docs/API.md).

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

extern crate alloc;

mod error;
mod graph;
mod id;

pub use error::ResolveError;
pub use graph::{ModuleGraph, Visibility};
pub use id::ModuleId;

// The foreign handle types this crate's API speaks in, re-exported so a consumer
// need not also name symbol-lang / source-lang just to call it.
pub use source_lang::SourceId;
pub use symbol_lang::Symbol;
