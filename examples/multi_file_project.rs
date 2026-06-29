//! Resolves names across a small multi-file project.
//!
//! Models four source files — `app`, `service`, `db`, and `util` — where `app`
//! pulls items in through `service`, which re-exports from `db`, and `db` keeps an
//! internal helper private. Run with:
//!
//! ```text
//! cargo run --example multi_file_project
//! ```

use intern_lang::Interner;
use module_lang::{ModuleGraph, ResolveError, Visibility};
use source_lang::SourceMap;

fn main() {
    // The interner mints a `Symbol` per name; the source map mints a `SourceId`
    // per file. Both are the handle types module-lang's API speaks in.
    let mut names = Interner::new();
    let mut sources = SourceMap::new();

    // One module per file. The payload here is the source line a name maps to;
    // a real front-end would store a definition id or an AST node instead.
    let mut graph: ModuleGraph<&str> = ModuleGraph::new();
    let app = graph.add_module(
        names.intern("app"),
        sources.add("app.lang", "").expect("fits"),
    );
    let service = graph.add_module(
        names.intern("service"),
        sources.add("service.lang", "").expect("fits"),
    );
    let db = graph.add_module(
        names.intern("db"),
        sources.add("db.lang", "").expect("fits"),
    );
    let util = graph.add_module(
        names.intern("util"),
        sources.add("util.lang", "").expect("fits"),
    );

    let connect = names.intern("connect");
    let query = names.intern("query");
    let escape = names.intern("escape");
    let log = names.intern("log");

    // `db` defines a public API and one private helper.
    graph
        .define(db, connect, Visibility::Public, "fn connect()")
        .expect("unique");
    graph
        .define(db, query, Visibility::Public, "fn query()")
        .expect("unique");
    graph
        .define(db, escape, Visibility::Private, "fn escape()")
        .expect("unique");

    // `util` exports a logger.
    graph
        .define(util, log, Visibility::Public, "fn log()")
        .expect("unique");

    // `service` re-exports `db::query` and imports the logger.
    graph.import(service, db, query).expect("unique");
    graph.import(service, util, log).expect("unique");

    // `app` imports through `service`.
    graph.import(app, service, query).expect("unique");
    graph.import(app, service, log).expect("unique");

    // A name resolves through the re-export chain to its origin.
    report("app::query", graph.resolve(app, query));
    report("app::log", graph.resolve(app, log));

    // `db` sees its own private helper; nobody else can import it.
    report("db::escape (from db)", graph.resolve(db, escape));
    graph.import(app, db, escape).expect("unique");
    report(
        "app::escape (imported, private)",
        graph.resolve(app, escape),
    );

    // A name that was never declared resolves to a defined error.
    let missing = names.intern("nonexistent");
    report("app::nonexistent", graph.resolve(app, missing));

    println!("\n{} modules resolved.", graph.len());
}

/// Prints one resolution outcome, turning a [`ResolveError`] back into a readable
/// line via the graph's ids.
fn report(label: &str, outcome: Result<&&str, ResolveError>) {
    match outcome {
        Ok(def) => println!("  {label:<34} -> {def}"),
        Err(err) => println!("  {label:<34} -> error: {err}"),
    }
}
