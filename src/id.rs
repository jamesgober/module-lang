//! Stable handles identifying a module within a graph.

/// A small, copyable handle to one module in a [`ModuleGraph`](crate::ModuleGraph).
///
/// A `ModuleId` is a 32-bit index minted by the graph when a module is added with
/// [`ModuleGraph::add_module`](crate::ModuleGraph::add_module). It is stable for
/// the life of the graph: the id returned for a module keeps pointing at that same
/// module no matter how many more are added afterwards, because modules are only
/// ever appended. That stability is what lets an import edge, an AST node, or a
/// cached diagnostic hold a `ModuleId` and resolve it later.
///
/// The id is deliberately opaque — there is no public constructor — so an id can
/// only come from the graph that actually holds the module it names. Passing an id
/// minted by a *different* graph to a query is a defined error
/// ([`ResolveError::UnknownModule`](crate::ResolveError::UnknownModule)), never a
/// panic or an out-of-bounds read.
///
/// # Examples
///
/// ```
/// use intern_lang::Interner;
/// use module_lang::ModuleGraph;
/// use source_lang::SourceMap;
///
/// let mut sources = SourceMap::new();
/// let mut names = Interner::new();
/// let mut graph: ModuleGraph<()> = ModuleGraph::new();
///
/// let a = graph.add_module(names.intern("a"), sources.add("a", "").expect("fits"));
/// let b = graph.add_module(names.intern("b"), sources.add("b", "").expect("fits"));
///
/// // Ids are assigned in order and stay distinct.
/// assert_eq!(a.to_u32(), 0);
/// assert_eq!(b.to_u32(), 1);
/// assert_ne!(a, b);
/// ```
///
/// With the `serde` feature it serialises transparently as its `u32` index, so a
/// handle stored in an AST node or an import table round-trips on its own.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(u32);

impl ModuleId {
    /// Wraps a raw index. Internal: only a graph may mint an id, so that every id
    /// in circulation names a module the graph actually holds.
    #[inline]
    pub(crate) const fn from_index(index: u32) -> Self {
        Self(index)
    }

    /// The index into the graph's module list this id wraps.
    #[inline]
    pub(crate) const fn to_index(self) -> usize {
        self.0 as usize
    }

    /// Returns the raw index this id wraps.
    ///
    /// The value is the module's insertion order, starting at `0`. It is useful as
    /// a dense array key — for a side table of per-module data — but the id itself
    /// should be preferred wherever an opaque handle will do.
    ///
    /// # Examples
    ///
    /// ```
    /// use intern_lang::Interner;
    /// use module_lang::ModuleGraph;
    /// use source_lang::SourceMap;
    ///
    /// let mut sources = SourceMap::new();
    /// let mut names = Interner::new();
    /// let mut graph: ModuleGraph<()> = ModuleGraph::new();
    /// let only = graph.add_module(names.intern("only"), sources.add("only", "").expect("fits"));
    /// assert_eq!(only.to_u32(), 0);
    /// ```
    #[inline]
    #[must_use]
    pub const fn to_u32(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_round_trips_through_its_index() {
        let id = ModuleId::from_index(7);
        assert_eq!(id.to_u32(), 7);
        assert_eq!(id.to_index(), 7);
    }

    #[test]
    fn test_ids_order_by_index() {
        assert!(ModuleId::from_index(1) < ModuleId::from_index(2));
        assert_eq!(ModuleId::from_index(3), ModuleId::from_index(3));
    }
}
