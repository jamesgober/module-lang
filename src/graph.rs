//! The module graph and name resolution.

use alloc::collections::BTreeMap;
use alloc::collections::btree_map::Entry as MapEntry;
use alloc::vec::Vec;

use source_lang::SourceId;
use symbol_lang::Symbol;

use crate::error::ResolveError;
use crate::id::ModuleId;

/// Whether an item is visible outside the module that declares it.
///
/// Visibility gates *cross-module* access only: a module can always reach its own
/// names, public or private, but an [`import`](ModuleGraph::import) from another
/// module reaches a name only if it is [`Public`](Self::Public). A
/// [`Private`](Self::Private) name reached through an import resolves to
/// [`ResolveError::Private`].
///
/// # Examples
///
/// ```
/// use intern_lang::Interner;
/// use module_lang::{ModuleGraph, ResolveError, Visibility};
/// use source_lang::SourceMap;
///
/// let mut sources = SourceMap::new();
/// let mut names = Interner::new();
/// let mut graph: ModuleGraph<()> = ModuleGraph::new();
///
/// let lib = graph.add_module(names.intern("lib"), sources.add("lib", "").expect("fits"));
/// let app = graph.add_module(names.intern("app"), sources.add("app", "").expect("fits"));
/// let secret = names.intern("secret");
///
/// graph.define(lib, secret, Visibility::Private, ())?;
///
/// // `lib` sees its own private name.
/// assert!(graph.resolve(lib, secret).is_ok());
///
/// // `app` imports it, but the import cannot reach a private name.
/// graph.import(app, lib, secret)?;
/// assert!(matches!(graph.resolve(app, secret), Err(ResolveError::Private { .. })));
/// # Ok::<(), module_lang::ResolveError>(())
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Resolvable from other modules through an import.
    Public,
    /// Resolvable only from within the module that declares it.
    Private,
}

/// One named entry in a module: either a definition made here, or a name imported
/// from another module under the same spelling.
#[derive(Clone, Debug)]
enum Entry<T> {
    /// A name defined in this module, with its visibility and payload.
    Define {
        /// Whether the definition is reachable from other modules.
        visibility: Visibility,
        /// The language-supplied payload the name resolves to.
        value: T,
    },
    /// A name brought into this module from another by `import`, keeping its
    /// spelling. Resolution follows `from` to where the name is defined.
    Import {
        /// The module the name is imported from.
        from: ModuleId,
    },
}

/// A single module: where it came from, and the names it declares.
#[derive(Clone, Debug)]
struct Module<T> {
    name: Symbol,
    source: SourceId,
    entries: BTreeMap<Symbol, Entry<T>>,
}

/// A set of modules and the import edges between them, against which names are
/// resolved.
///
/// `ModuleGraph<T>` is the owner of resolution state. Each module is added with a
/// name and the [`SourceId`] of the file it came from, then has names
/// [`define`](Self::define)d in it and names [`import`](Self::import)ed into it
/// from other modules. [`resolve`](Self::resolve) answers the question every
/// `use`/`import` asks — *which item does this name refer to?* — returning the
/// language-supplied payload `T`, or a [`ResolveError`] for a name that is missing,
/// private, or part of an import cycle.
///
/// `T` is whatever the language wants a name to resolve to — a definition id, an
/// AST node handle, a type — and carries no trait bounds. A module's namespace is
/// flat: a name is declared at most once per module, whether by definition or
/// import. Names are keyed in a [`BTreeMap`], so a lookup compares interned-symbol
/// integers in `O(log items)` and iteration is deterministic.
///
/// # Examples
///
/// ```
/// use intern_lang::Interner;
/// use module_lang::{ModuleGraph, Visibility};
/// use source_lang::SourceMap;
///
/// // Two files, each a module.
/// let mut sources = SourceMap::new();
/// let app_src = sources.add("app.lang", "use util.helper;").expect("fits");
/// let util_src = sources.add("util.lang", "pub fn helper() {}").expect("fits");
///
/// let mut names = Interner::new();
/// let helper = names.intern("helper");
///
/// // A module per file; `util` exports `helper`; `app` imports it.
/// let mut graph: ModuleGraph<&str> = ModuleGraph::new();
/// let app = graph.add_module(names.intern("app"), app_src);
/// let util = graph.add_module(names.intern("util"), util_src);
/// graph.define(util, helper, Visibility::Public, "fn helper")?;
/// graph.import(app, util, helper)?;
///
/// // `helper` resolves from `app` through the import to its definition in `util`.
/// assert_eq!(graph.resolve(app, helper), Ok(&"fn helper"));
/// # Ok::<(), module_lang::ResolveError>(())
/// ```
#[derive(Clone, Debug)]
pub struct ModuleGraph<T> {
    modules: Vec<Module<T>>,
}

impl<T> Default for ModuleGraph<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ModuleGraph<T> {
    /// Creates an empty graph.
    ///
    /// # Examples
    ///
    /// ```
    /// use module_lang::ModuleGraph;
    ///
    /// let graph: ModuleGraph<()> = ModuleGraph::new();
    /// assert!(graph.is_empty());
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    /// Creates an empty graph with room for `modules` modules before reallocating.
    ///
    /// A hint only: the graph still grows as needed. Useful when the file count is
    /// known up front, to add every module without an intermediate reallocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use module_lang::ModuleGraph;
    ///
    /// let graph: ModuleGraph<()> = ModuleGraph::with_capacity(64);
    /// assert!(graph.is_empty());
    /// ```
    #[must_use]
    pub fn with_capacity(modules: usize) -> Self {
        Self {
            modules: Vec::with_capacity(modules),
        }
    }

    /// Adds a module with a display `name` and the `source` it was read from,
    /// returning its stable [`ModuleId`].
    ///
    /// The id is the module's insertion order and never changes; hold it to define
    /// names in the module, import from it, or resolve against it. The `name` is
    /// for diagnostics and the `source` records which file the module came from —
    /// neither participates in resolution, which is always by id.
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
    /// let src = sources.add("m.lang", "").expect("fits");
    /// let m = graph.add_module(names.intern("m"), src);
    /// assert_eq!(graph.len(), 1);
    /// assert_eq!(graph.module_source(m), Some(src));
    /// ```
    pub fn add_module(&mut self, name: Symbol, source: SourceId) -> ModuleId {
        let index = self.modules.len();
        // A `Vec` of modules exhausts memory long before this index could overflow
        // `u32` (each module is many bytes; `u32::MAX` of them is far past any real
        // address space), so the cast below cannot wrap in practice.
        debug_assert!(
            index < u32::MAX as usize,
            "module count exceeds u32 addressing"
        );
        self.modules.push(Module {
            name,
            source,
            entries: BTreeMap::new(),
        });
        ModuleId::from_index(index as u32)
    }

    /// Defines `name` in `module` with a visibility and a payload.
    ///
    /// Returns [`ResolveError::DuplicateName`] if the module already declares the
    /// name (by an earlier definition or an import), and
    /// [`ResolveError::UnknownModule`] if `module` was not minted by this graph. On
    /// success the name resolves to `value` from within the module immediately, and
    /// from other modules that import it when it is [`Visibility::Public`].
    ///
    /// # Examples
    ///
    /// ```
    /// use intern_lang::Interner;
    /// use module_lang::{ModuleGraph, ResolveError, Visibility};
    /// use source_lang::SourceMap;
    ///
    /// let mut sources = SourceMap::new();
    /// let mut names = Interner::new();
    /// let mut graph: ModuleGraph<i32> = ModuleGraph::new();
    ///
    /// let m = graph.add_module(names.intern("m"), sources.add("m", "").expect("fits"));
    /// let x = names.intern("x");
    ///
    /// graph.define(m, x, Visibility::Public, 42)?;
    /// assert_eq!(graph.resolve(m, x), Ok(&42));
    ///
    /// // A second definition of the same name in the same module is rejected.
    /// assert!(matches!(
    ///     graph.define(m, x, Visibility::Private, 7),
    ///     Err(ResolveError::DuplicateName { .. }),
    /// ));
    /// # Ok::<(), module_lang::ResolveError>(())
    /// ```
    pub fn define(
        &mut self,
        module: ModuleId,
        name: Symbol,
        visibility: Visibility,
        value: T,
    ) -> Result<(), ResolveError> {
        let target = self.module_mut(module)?;
        match target.entries.entry(name) {
            MapEntry::Occupied(_) => Err(ResolveError::DuplicateName { module, name }),
            MapEntry::Vacant(slot) => {
                let _ = slot.insert(Entry::Define { visibility, value });
                Ok(())
            }
        }
    }

    /// Imports `name` into `into` from the `from` module, keeping its spelling.
    ///
    /// This records an import edge; it does not require `name` to be defined in
    /// `from` yet, so a graph can be built in any order. Whether the import
    /// actually reaches a public definition is determined when the name is
    /// [`resolve`](Self::resolve)d.
    ///
    /// Returns [`ResolveError::DuplicateName`] if `into` already declares `name`,
    /// and [`ResolveError::UnknownModule`] if either id was not minted by this
    /// graph.
    ///
    /// # Examples
    ///
    /// ```
    /// use intern_lang::Interner;
    /// use module_lang::{ModuleGraph, Visibility};
    /// use source_lang::SourceMap;
    ///
    /// let mut sources = SourceMap::new();
    /// let mut names = Interner::new();
    /// let mut graph: ModuleGraph<()> = ModuleGraph::new();
    ///
    /// let lib = graph.add_module(names.intern("lib"), sources.add("lib", "").expect("fits"));
    /// let app = graph.add_module(names.intern("app"), sources.add("app", "").expect("fits"));
    /// let item = names.intern("item");
    ///
    /// graph.define(lib, item, Visibility::Public, ())?;
    /// graph.import(app, lib, item)?;
    /// assert!(graph.resolve(app, item).is_ok());
    /// # Ok::<(), module_lang::ResolveError>(())
    /// ```
    pub fn import(
        &mut self,
        into: ModuleId,
        from: ModuleId,
        name: Symbol,
    ) -> Result<(), ResolveError> {
        if from.to_index() >= self.modules.len() {
            return Err(ResolveError::UnknownModule(from));
        }
        let target = self.module_mut(into)?;
        match target.entries.entry(name) {
            MapEntry::Occupied(_) => Err(ResolveError::DuplicateName { module: into, name }),
            MapEntry::Vacant(slot) => {
                let _ = slot.insert(Entry::Import { from });
                Ok(())
            }
        }
    }

    /// Resolves `name` as seen from within `module`, returning the payload it
    /// refers to.
    ///
    /// A name defined in the module wins, public or private. A name imported into
    /// the module is followed to its source module — and onward along any chain of
    /// re-imports — to the public definition it names. The failure modes are all
    /// defined [`ResolveError`]s:
    ///
    /// - [`Unresolved`](ResolveError::Unresolved) — the name is declared nowhere on
    ///   the path.
    /// - [`Private`](ResolveError::Private) — an import reached a name private to
    ///   another module.
    /// - [`ImportCycle`](ResolveError::ImportCycle) — the import chain loops.
    /// - [`UnknownModule`](ResolveError::UnknownModule) — `module` is not from this
    ///   graph.
    ///
    /// # Examples
    ///
    /// ```
    /// use intern_lang::Interner;
    /// use module_lang::{ModuleGraph, ResolveError, Visibility};
    /// use source_lang::SourceMap;
    ///
    /// let mut sources = SourceMap::new();
    /// let mut names = Interner::new();
    /// let mut graph: ModuleGraph<&str> = ModuleGraph::new();
    ///
    /// let m = graph.add_module(names.intern("m"), sources.add("m", "").expect("fits"));
    /// let here = names.intern("here");
    /// let gone = names.intern("gone");
    ///
    /// graph.define(m, here, Visibility::Public, "local")?;
    /// assert_eq!(graph.resolve(m, here), Ok(&"local"));
    /// assert!(matches!(graph.resolve(m, gone), Err(ResolveError::Unresolved { .. })));
    /// # Ok::<(), module_lang::ResolveError>(())
    /// ```
    pub fn resolve(&self, module: ModuleId, name: Symbol) -> Result<&T, ResolveError> {
        let source = self.module(module)?;
        let from = match source.entries.get(&name) {
            None => return Err(ResolveError::Unresolved { module, name }),
            // A name defined here resolves directly, regardless of visibility: a
            // module can always see its own names.
            Some(Entry::Define { value, .. }) => return Ok(value),
            Some(Entry::Import { from }) => *from,
        };
        let mut visited = Vec::new();
        visited.push((module.to_u32(), name.as_u32()));
        self.resolve_export(from, name, &mut visited)
    }

    /// Number of modules in the graph.
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
    /// assert_eq!(graph.len(), 0);
    /// graph.add_module(names.intern("m"), sources.add("m", "").expect("fits"));
    /// assert_eq!(graph.len(), 1);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Whether the graph holds no modules.
    ///
    /// # Examples
    ///
    /// ```
    /// use module_lang::ModuleGraph;
    ///
    /// let graph: ModuleGraph<()> = ModuleGraph::new();
    /// assert!(graph.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// The display name a module was added with, or `None` if `module` is not from
    /// this graph.
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
    /// let name = names.intern("m");
    /// let m = graph.add_module(name, sources.add("m", "").expect("fits"));
    /// assert_eq!(graph.module_name(m), Some(name));
    /// ```
    #[must_use]
    pub fn module_name(&self, module: ModuleId) -> Option<Symbol> {
        self.modules.get(module.to_index()).map(|m| m.name)
    }

    /// The [`SourceId`] a module was added with, or `None` if `module` is not from
    /// this graph.
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
    /// let src = sources.add("m", "").expect("fits");
    /// let m = graph.add_module(names.intern("m"), src);
    /// assert_eq!(graph.module_source(m), Some(src));
    /// ```
    #[must_use]
    pub fn module_source(&self, module: ModuleId) -> Option<SourceId> {
        self.modules.get(module.to_index()).map(|m| m.source)
    }

    /// Borrows a module by id, or reports it as unknown.
    fn module(&self, id: ModuleId) -> Result<&Module<T>, ResolveError> {
        self.modules
            .get(id.to_index())
            .ok_or(ResolveError::UnknownModule(id))
    }

    /// Borrows a module mutably by id, or reports it as unknown.
    fn module_mut(&mut self, id: ModuleId) -> Result<&mut Module<T>, ResolveError> {
        self.modules
            .get_mut(id.to_index())
            .ok_or(ResolveError::UnknownModule(id))
    }

    /// Resolves `name` as a public export of `start`, following any chain of
    /// re-imports to the definition it names.
    ///
    /// `visited` carries the `(module, name)` pairs already seen so a chain that
    /// loops back is reported as a cycle rather than recursing without end. The
    /// walk is iterative and copies module ids out of the graph (phase one) so the
    /// terminal definition can be borrowed in a single step (phase two) without
    /// returning a borrow across the loop.
    fn resolve_export(
        &self,
        start: ModuleId,
        name: Symbol,
        visited: &mut Vec<(u32, u32)>,
    ) -> Result<&T, ResolveError> {
        let mut module = start;
        let terminal = loop {
            let key = (module.to_u32(), name.as_u32());
            if visited.contains(&key) {
                return Err(ResolveError::ImportCycle { module, name });
            }
            visited.push(key);
            match self.module(module)?.entries.get(&name) {
                None => return Err(ResolveError::Unresolved { module, name }),
                Some(Entry::Define {
                    visibility: Visibility::Public,
                    ..
                }) => break module,
                Some(Entry::Define {
                    visibility: Visibility::Private,
                    ..
                }) => return Err(ResolveError::Private { module, name }),
                Some(Entry::Import { from }) => module = *from,
            }
        };
        match self.module(terminal)?.entries.get(&name) {
            Some(Entry::Define { value, .. }) => Ok(value),
            // Phase one ended on a public definition for `(terminal, name)` and the
            // graph cannot have changed through `&self`, so this arm does not occur
            // in practice; it returns a defined error rather than panicking.
            _ => Err(ResolveError::Unresolved {
                module: terminal,
                name,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use intern_lang::Interner;
    use source_lang::SourceMap;

    use super::*;

    /// Builds a graph plus the interner and source map its tests share.
    fn fixture() -> (ModuleGraph<u32>, Interner, SourceMap) {
        (ModuleGraph::new(), Interner::new(), SourceMap::new())
    }

    #[test]
    fn test_add_module_assigns_sequential_stable_ids() {
        let (mut g, mut n, mut s) = fixture();
        let a = g.add_module(n.intern("a"), s.add("a", "").expect("fits"));
        let b = g.add_module(n.intern("b"), s.add("b", "").expect("fits"));
        assert_eq!(a.to_u32(), 0);
        assert_eq!(b.to_u32(), 1);
        assert_eq!(g.len(), 2);
        // The first id still names the first module after more are added.
        assert_eq!(g.module_name(a), Some(n.intern("a")));
    }

    #[test]
    fn test_define_then_resolve_returns_value() {
        let (mut g, mut n, mut s) = fixture();
        let m = g.add_module(n.intern("m"), s.add("m", "").expect("fits"));
        let x = n.intern("x");
        g.define(m, x, Visibility::Public, 99).expect("unique");
        assert_eq!(g.resolve(m, x), Ok(&99));
    }

    #[test]
    fn test_define_duplicate_in_same_module_is_error() {
        let (mut g, mut n, mut s) = fixture();
        let m = g.add_module(n.intern("m"), s.add("m", "").expect("fits"));
        let x = n.intern("x");
        g.define(m, x, Visibility::Public, 1)
            .expect("first is unique");
        assert_eq!(
            g.define(m, x, Visibility::Public, 2),
            Err(ResolveError::DuplicateName { module: m, name: x }),
        );
    }

    #[test]
    fn test_import_resolves_through_to_definition() {
        let (mut g, mut n, mut s) = fixture();
        let lib = g.add_module(n.intern("lib"), s.add("lib", "").expect("fits"));
        let app = g.add_module(n.intern("app"), s.add("app", "").expect("fits"));
        let item = n.intern("item");
        g.define(lib, item, Visibility::Public, 7).expect("unique");
        g.import(app, lib, item).expect("unique");
        assert_eq!(g.resolve(app, item), Ok(&7));
    }

    #[test]
    fn test_resolve_unknown_name_is_unresolved() {
        let (mut g, mut n, mut s) = fixture();
        let m = g.add_module(n.intern("m"), s.add("m", "").expect("fits"));
        let gone = n.intern("gone");
        assert_eq!(
            g.resolve(m, gone),
            Err(ResolveError::Unresolved {
                module: m,
                name: gone
            }),
        );
    }

    #[test]
    fn test_private_definition_is_local_only() {
        let (mut g, mut n, mut s) = fixture();
        let lib = g.add_module(n.intern("lib"), s.add("lib", "").expect("fits"));
        let app = g.add_module(n.intern("app"), s.add("app", "").expect("fits"));
        let secret = n.intern("secret");
        g.define(lib, secret, Visibility::Private, 5)
            .expect("unique");
        // Visible at home.
        assert_eq!(g.resolve(lib, secret), Ok(&5));
        // Not through an import.
        g.import(app, lib, secret).expect("unique");
        assert_eq!(
            g.resolve(app, secret),
            Err(ResolveError::Private {
                module: lib,
                name: secret
            }),
        );
    }

    #[test]
    fn test_import_cycle_is_reported_not_looped() {
        let (mut g, mut n, mut s) = fixture();
        let a = g.add_module(n.intern("a"), s.add("a", "").expect("fits"));
        let b = g.add_module(n.intern("b"), s.add("b", "").expect("fits"));
        let x = n.intern("x");
        // a imports x from b, b imports x from a: a closed loop with no definition.
        g.import(a, b, x).expect("unique");
        g.import(b, a, x).expect("unique");
        assert!(matches!(
            g.resolve(a, x),
            Err(ResolveError::ImportCycle { .. })
        ));
    }

    #[test]
    fn test_self_import_is_a_cycle() {
        let (mut g, mut n, mut s) = fixture();
        let a = g.add_module(n.intern("a"), s.add("a", "").expect("fits"));
        let x = n.intern("x");
        g.import(a, a, x).expect("unique");
        assert!(matches!(
            g.resolve(a, x),
            Err(ResolveError::ImportCycle { .. })
        ));
    }

    #[test]
    fn test_unknown_module_id_is_error_not_panic() {
        let (mut g, mut n, mut s) = fixture();
        let real = g.add_module(n.intern("a"), s.add("a", "").expect("fits"));
        // An id from a second, separate graph does not index this one.
        let mut other: ModuleGraph<u32> = ModuleGraph::new();
        let mut s2 = SourceMap::new();
        let foreign = other.add_module(n.intern("z"), s2.add("z", "").expect("fits"));
        let _ = foreign;
        // `real` has index 0; an out-of-range id is rejected.
        let beyond = ModuleId::from_index(g.len() as u32 + 5);
        let x = n.intern("x");
        assert_eq!(
            g.resolve(beyond, x),
            Err(ResolveError::UnknownModule(beyond))
        );
        assert!(g.resolve(real, x).is_err());
    }

    #[test]
    fn test_re_export_chain_resolves_to_origin() {
        let (mut g, mut n, mut s) = fixture();
        let c = g.add_module(n.intern("c"), s.add("c", "").expect("fits"));
        let b = g.add_module(n.intern("b"), s.add("b", "").expect("fits"));
        let a = g.add_module(n.intern("a"), s.add("a", "").expect("fits"));
        let item = n.intern("item");
        g.define(c, item, Visibility::Public, 123).expect("unique");
        g.import(b, c, item).expect("unique"); // b re-exports c::item
        g.import(a, b, item).expect("unique"); // a imports through b
        assert_eq!(g.resolve(a, item), Ok(&123));
    }
}
