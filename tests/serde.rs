//! Round-trips the serde-enabled metadata types through `serde_json`. Compiled
//! only with the `serde` feature; otherwise the whole file is empty.
#![cfg(feature = "serde")]

use intern_lang::Interner;
use module_lang::{ModuleGraph, ModuleId, Visibility};
use source_lang::SourceMap;

#[test]
fn test_module_id_round_trips_as_its_index() {
    let mut graph: ModuleGraph<()> = ModuleGraph::new();
    let mut names = Interner::new();
    let mut sources = SourceMap::new();
    let id = graph.add_module(names.intern("m"), sources.add("m", "").expect("fits"));

    // A `ModuleId` is a newtype over its `u32` index and serialises transparently.
    let json = serde_json::to_string(&id).expect("serialize");
    assert_eq!(json, "0");

    let back: ModuleId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(id, back);
}

#[test]
fn test_visibility_round_trips() {
    for vis in [Visibility::Public, Visibility::Private] {
        let json = serde_json::to_string(&vis).expect("serialize");
        let back: Visibility = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(vis, back);
    }
    assert_eq!(
        serde_json::to_string(&Visibility::Public).expect("serialize"),
        "\"Public\""
    );
}
