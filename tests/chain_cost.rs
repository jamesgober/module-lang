//! Resolving through a long import chain must cost about linear time in the
//! chain length, not quadratic.
//!
//! The test compares the fastest of several runs at two chain lengths eight
//! times apart. Linear (or `L log L`) work grows by roughly 8 to 12 times;
//! quadratic work grows by about 64 times. Comparing a ratio rather than an
//! absolute time keeps the test stable on a slow or busy machine.

use std::time::{Duration, Instant};

use intern_lang::Interner;
use module_lang::{ModuleGraph, Visibility};
use source_lang::SourceMap;

/// Fastest of `runs` resolves from the head of a chain of `len` modules.
fn fastest_resolve(len: usize, runs: usize) -> Duration {
    let mut names = Interner::new();
    let mut sources = SourceMap::new();
    let src = sources.add("m", "").expect("fits");
    let x = names.intern("x");
    let mut graph: ModuleGraph<u32> = ModuleGraph::new();
    let mods: Vec<_> = (0..len)
        .map(|i| graph.add_module(names.intern(&format!("m{i}")), src))
        .collect();
    graph
        .define(mods[len - 1], x, Visibility::Public, 7)
        .expect("defines");
    for pair in mods.windows(2) {
        graph.import(pair[0], pair[1], x).expect("imports");
    }
    (0..runs)
        .map(|_| {
            let start = Instant::now();
            assert_eq!(graph.resolve(mods[0], x), Ok(&7));
            start.elapsed()
        })
        .min()
        .unwrap_or_default()
}

#[test]
fn long_import_chain_resolves_in_linear_time() {
    let short = fastest_resolve(4_000, 7);
    let long = fastest_resolve(32_000, 7);
    // Avoid dividing by a timer reading of zero on a very fast machine.
    let short_ns = short.as_nanos().max(1);
    let ratio = long.as_nanos() / short_ns;
    assert!(
        ratio < 30,
        "8x longer chain took {ratio}x as long ({short:?} vs {long:?})"
    );
}

#[test]
fn cycle_is_still_reported_at_the_repeated_module() {
    let mut names = Interner::new();
    let mut sources = SourceMap::new();
    let src = sources.add("m", "").expect("fits");
    let x = names.intern("x");
    let mut graph: ModuleGraph<u32> = ModuleGraph::new();
    let mods: Vec<_> = (0..200)
        .map(|i| graph.add_module(names.intern(&format!("m{i}")), src))
        .collect();
    for pair in mods.windows(2) {
        graph.import(pair[0], pair[1], x).expect("imports");
    }
    // Close the loop back onto module 50.
    graph.import(mods[199], mods[50], x).expect("imports");
    assert_eq!(
        graph.resolve(mods[0], x),
        Err(module_lang::ResolveError::ImportCycle {
            module: mods[50],
            name: x,
        })
    );
}
