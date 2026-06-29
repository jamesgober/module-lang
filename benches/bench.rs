//! Criterion benchmarks for the resolution hot paths: building a graph, a local
//! definition hit, a walk along an import chain, and a miss.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use intern_lang::Interner;
use module_lang::{ModuleGraph, ModuleId, Symbol, Visibility};
use source_lang::SourceMap;

/// Builds a graph of `modules` modules, each defining the same `items` public
/// names. Returns the graph and the id/name pools to probe it with.
fn dense_graph(modules: usize, items: usize) -> (ModuleGraph<u32>, Vec<ModuleId>, Vec<Symbol>) {
    let mut interner = Interner::new();
    let mut sources = SourceMap::new();
    let names: Vec<Symbol> = (0..items)
        .map(|i| interner.intern(&format!("item{i}")))
        .collect();

    let mut graph = ModuleGraph::with_capacity(modules);
    let ids: Vec<ModuleId> = (0..modules)
        .map(|i| {
            let src = sources.add(format!("m{i}"), "").expect("source fits");
            graph.add_module(interner.intern(&format!("mod{i}")), src)
        })
        .collect();

    for &id in &ids {
        for (value, &name) in names.iter().enumerate() {
            graph
                .define(id, name, Visibility::Public, value as u32)
                .expect("names are unique within a module");
        }
    }
    (graph, ids, names)
}

/// Builds a re-export chain `depth` modules long: the last defines `item`, every
/// earlier module re-exports the next. Returns the graph, the head module, and the
/// chained name.
fn import_chain(depth: usize) -> (ModuleGraph<u32>, ModuleId, Symbol) {
    let mut interner = Interner::new();
    let mut sources = SourceMap::new();
    let item = interner.intern("item");

    let mut graph = ModuleGraph::with_capacity(depth);
    let ids: Vec<ModuleId> = (0..depth)
        .map(|i| {
            let src = sources.add(format!("m{i}"), "").expect("source fits");
            graph.add_module(interner.intern(&format!("mod{i}")), src)
        })
        .collect();

    graph
        .define(ids[depth - 1], item, Visibility::Public, 1)
        .expect("unique");
    for i in 0..depth - 1 {
        graph.import(ids[i], ids[i + 1], item).expect("unique");
    }
    (graph, ids[0], item)
}

fn bench_build(c: &mut Criterion) {
    c.bench_function("build_64x64", |b| {
        b.iter(|| dense_graph(black_box(64), black_box(64)));
    });
}

fn bench_resolve_local_hit(c: &mut Criterion) {
    let (graph, ids, names) = dense_graph(64, 64);
    let module = ids[32];
    let name = names[32];
    c.bench_function("resolve_local_hit", |b| {
        b.iter(|| graph.resolve(black_box(module), black_box(name)));
    });
}

fn bench_resolve_miss(c: &mut Criterion) {
    let (graph, ids, _names) = dense_graph(64, 64);
    let module = ids[32];
    let mut interner = Interner::new();
    let absent = interner.intern("definitely-not-defined");
    c.bench_function("resolve_miss", |b| {
        b.iter(|| graph.resolve(black_box(module), black_box(absent)));
    });
}

fn bench_resolve_chain(c: &mut Criterion) {
    let (graph, head, name) = import_chain(32);
    c.bench_function("resolve_import_chain_32", |b| {
        b.iter(|| graph.resolve(black_box(head), black_box(name)));
    });
}

criterion_group!(
    benches,
    bench_build,
    bench_resolve_local_hit,
    bench_resolve_miss,
    bench_resolve_chain,
);
criterion_main!(benches);
