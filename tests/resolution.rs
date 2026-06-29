//! Property tests for the resolution invariants in `dev/DIRECTIVES.md`.
//!
//! A random sequence of `define`/`import` operations is applied to both a
//! [`ModuleGraph`] and an independent, deliberately naive reference model (a
//! `BTreeMap` per module plus a recursive resolver with a `HashSet` visited set).
//! Every `(module, name)` pair is then resolved in both and the classifications
//! compared, so the optimised iterative resolver is held to the obvious one across
//! a wide input space — including the import cycles, private targets, and missing
//! names that random graphs produce.

use std::collections::{BTreeMap, HashSet};

use intern_lang::Interner;
use module_lang::{ModuleGraph, ModuleId, ResolveError, Symbol, Visibility};
use proptest::prelude::*;
use source_lang::SourceMap;

const MODULES: usize = 5;
const NAMES: usize = 5;

#[derive(Clone, Debug)]
enum Op {
    Define {
        module: usize,
        name: usize,
        public: bool,
        value: u32,
    },
    Import {
        into: usize,
        from: usize,
        name: usize,
    },
}

/// One entry in the reference model, mirroring the graph's internal entry.
#[derive(Clone, Debug)]
enum RefEntry {
    Define { public: bool, value: u32 },
    Import { from: usize },
}

/// The outcome of resolving a name, collapsed to compare the two resolvers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Found(u32),
    Unresolved,
    Private,
    Cycle,
}

/// Naive resolver: a name defined in the module wins (any visibility); an import
/// is followed recursively with visibility enforced and a `HashSet` guarding
/// against cycles.
fn ref_resolve(model: &[BTreeMap<usize, RefEntry>], module: usize, name: usize) -> Class {
    match model[module].get(&name) {
        None => Class::Unresolved,
        Some(RefEntry::Define { value, .. }) => Class::Found(*value),
        Some(RefEntry::Import { from }) => {
            let mut visited = HashSet::new();
            let _ = visited.insert((module, name));
            ref_export(model, *from, name, &mut visited)
        }
    }
}

fn ref_export(
    model: &[BTreeMap<usize, RefEntry>],
    module: usize,
    name: usize,
    visited: &mut HashSet<(usize, usize)>,
) -> Class {
    if !visited.insert((module, name)) {
        return Class::Cycle;
    }
    match model[module].get(&name) {
        None => Class::Unresolved,
        Some(RefEntry::Define {
            public: true,
            value,
        }) => Class::Found(*value),
        Some(RefEntry::Define { public: false, .. }) => Class::Private,
        Some(RefEntry::Import { from }) => ref_export(model, *from, name, visited),
    }
}

/// The graph's resolution result, collapsed to the same classification.
fn real_resolve(graph: &ModuleGraph<u32>, module: ModuleId, name: Symbol) -> Class {
    match graph.resolve(module, name) {
        Ok(value) => Class::Found(*value),
        Err(ResolveError::Unresolved { .. }) => Class::Unresolved,
        Err(ResolveError::Private { .. }) => Class::Private,
        Err(ResolveError::ImportCycle { .. }) => Class::Cycle,
        Err(other) => panic!("resolve produced an unexpected error: {other:?}"),
    }
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0..MODULES, 0..NAMES, any::<bool>(), any::<u32>()).prop_map(
            |(module, name, public, value)| Op::Define {
                module,
                name,
                public,
                value,
            }
        ),
        (0..MODULES, 0..MODULES, 0..NAMES).prop_map(|(into, from, name)| Op::Import {
            into,
            from,
            name
        }),
    ]
}

proptest! {
    #[test]
    fn resolution_matches_naive_reference(ops in prop::collection::vec(op_strategy(), 0..40)) {
        let mut interner = Interner::new();
        let names: Vec<Symbol> = (0..NAMES).map(|i| interner.intern(&format!("n{i}"))).collect();

        let mut sources = SourceMap::new();
        let mut graph: ModuleGraph<u32> = ModuleGraph::new();
        let ids: Vec<ModuleId> = (0..MODULES)
            .map(|i| {
                let src = sources.add(format!("m{i}"), "").expect("source fits");
                graph.add_module(interner.intern(&format!("mod{i}")), src)
            })
            .collect();
        let mut model: Vec<BTreeMap<usize, RefEntry>> = vec![BTreeMap::new(); MODULES];

        // Apply each operation to both, asserting the duplicate-name contract:
        // an add succeeds exactly when the name was not already declared.
        for op in ops {
            match op {
                Op::Define { module, name, public, value } => {
                    let absent = !model[module].contains_key(&name);
                    let vis = if public { Visibility::Public } else { Visibility::Private };
                    let result = graph.define(ids[module], names[name], vis, value);
                    prop_assert_eq!(result.is_ok(), absent);
                    if absent {
                        let _ = model[module].insert(name, RefEntry::Define { public, value });
                    }
                }
                Op::Import { into, from, name } => {
                    let absent = !model[into].contains_key(&name);
                    let result = graph.import(ids[into], ids[from], names[name]);
                    prop_assert_eq!(result.is_ok(), absent);
                    if absent {
                        let _ = model[into].insert(name, RefEntry::Import { from });
                    }
                }
            }
        }

        // Invariant: ids are unique, stable, and equal to insertion order.
        for (index, id) in ids.iter().enumerate() {
            prop_assert_eq!(id.to_u32() as usize, index);
        }

        // Invariant: resolution agrees with the reference for every pair, and is
        // deterministic (resolving twice gives the same answer).
        for (module, &module_id) in ids.iter().enumerate() {
            for (name, &name_sym) in names.iter().enumerate() {
                let expected = ref_resolve(&model, module, name);
                let actual = real_resolve(&graph, module_id, name_sym);
                prop_assert_eq!(actual, expected, "module {} name {}", module, name);
                prop_assert_eq!(real_resolve(&graph, module_id, name_sym), actual);
            }
        }
    }
}

/// A worst-case re-export chain resolves to its origin and never overflows the
/// stack: the walk is iterative, so a long chain is linear work, not deep
/// recursion.
#[test]
fn test_long_reexport_chain_resolves_to_origin() {
    const DEPTH: usize = 4_096;

    let mut interner = Interner::new();
    let mut sources = SourceMap::new();
    let mut graph: ModuleGraph<u32> = ModuleGraph::new();
    let item = interner.intern("item");

    let ids: Vec<ModuleId> = (0..DEPTH)
        .map(|i| {
            let src = sources.add(format!("m{i}"), "").expect("source fits");
            graph.add_module(interner.intern(&format!("mod{i}")), src)
        })
        .collect();

    // The last module defines `item`; every earlier module re-exports the next.
    graph
        .define(ids[DEPTH - 1], item, Visibility::Public, 777)
        .expect("unique");
    for i in 0..DEPTH - 1 {
        graph.import(ids[i], ids[i + 1], item).expect("unique");
    }

    assert_eq!(graph.resolve(ids[0], item), Ok(&777));
}
