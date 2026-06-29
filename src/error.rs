//! The error type returned when the graph is built or a name is resolved.

use core::fmt;

use symbol_lang::Symbol;

use crate::id::ModuleId;

/// The reason a graph operation or a name resolution did not succeed.
///
/// Building the graph and resolving through it can fail five ways, each a
/// distinct, defined outcome rather than a panic or a silently wrong answer:
///
/// - the id passed in was not minted by this graph
///   ([`UnknownModule`](Self::UnknownModule)),
/// - a name was declared twice in one module
///   ([`DuplicateName`](Self::DuplicateName)),
/// - a name is not declared in the module it was looked up in
///   ([`Unresolved`](Self::Unresolved)),
/// - an import targets a name that is private to the module that declares it
///   ([`Private`](Self::Private)), or
/// - an import chain loops back on itself ([`ImportCycle`](Self::ImportCycle)).
///
/// Each variant carries the [`ModuleId`] and [`Symbol`] it concerns, so a caller
/// can recover the human-readable spelling from its own interner and build a
/// richer diagnostic. The [`fmt::Display`] form is a terse fallback that names the
/// raw ids — module-lang does not own the interner, so it cannot print the name.
///
/// The enum is `#[non_exhaustive]`: a downstream `match` must include a wildcard
/// arm, so later additions never force a breaking change on callers.
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
/// let m = graph.add_module(names.intern("m"), sources.add("m", "").expect("fits"));
/// let x = names.intern("x");
/// graph.define(m, x, Visibility::Public, ()).expect("first definition is unique");
///
/// // Declaring the same name again in the same module is a defined error.
/// assert!(matches!(
///     graph.define(m, x, Visibility::Public, ()),
///     Err(ResolveError::DuplicateName { .. }),
/// ));
///
/// // Looking up a name that was never declared is a defined error, not a panic.
/// let y = names.intern("y");
/// assert!(matches!(
///     graph.resolve(m, y),
///     Err(ResolveError::Unresolved { .. }),
/// ));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResolveError {
    /// A [`ModuleId`] was passed that this graph never minted.
    ///
    /// Every id comes from
    /// [`ModuleGraph::add_module`](crate::ModuleGraph::add_module) on the graph it
    /// is used with; an id from a different graph, or one fabricated by other
    /// means, lands here instead of indexing out of bounds.
    UnknownModule(ModuleId),

    /// A name was declared twice in the same module.
    ///
    /// Both [`define`](crate::ModuleGraph::define) and
    /// [`import`](crate::ModuleGraph::import) reject a name already present in the
    /// target module — a module's namespace is flat, so a definition and an import
    /// cannot share a spelling. The conflict is reported where the second
    /// declaration is added, not silently resolved last-write-wins.
    DuplicateName {
        /// The module the duplicate was added to.
        module: ModuleId,
        /// The name that was already declared there.
        name: Symbol,
    },

    /// A name is not declared in the module it was looked up in.
    ///
    /// Returned by [`resolve`](crate::ModuleGraph::resolve) when neither a
    /// definition nor an import in the module (or, following an import chain, in a
    /// source module) declares the name.
    Unresolved {
        /// The module the name was looked up in.
        module: ModuleId,
        /// The name that could not be found.
        name: Symbol,
    },

    /// An import resolved to a name that is private to the module declaring it.
    ///
    /// A [`Visibility::Private`](crate::Visibility::Private) definition is visible
    /// only from within its own module; reaching it through an import from another
    /// module is rejected here rather than silently allowed.
    Private {
        /// The module that declares the name privately.
        module: ModuleId,
        /// The private name an import tried to reach.
        name: Symbol,
    },

    /// An import chain loops back on itself.
    ///
    /// Resolving an import follows it to its source module, and so on; if that walk
    /// returns to a `(module, name)` pair it has already visited, the chain forms a
    /// cycle. It is reported here in bounded time rather than recursing without
    /// limit.
    ImportCycle {
        /// The module the walk looped back to.
        module: ModuleId,
        /// The name whose import chain forms the cycle.
        name: Symbol,
    },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownModule(id) => {
                write!(f, "module id {} was not minted by this graph", id.to_u32())
            }
            Self::DuplicateName { module, name } => write!(
                f,
                "name (symbol {}) is already declared in module {}",
                name.as_u32(),
                module.to_u32(),
            ),
            Self::Unresolved { module, name } => write!(
                f,
                "no name (symbol {}) is declared in module {}",
                name.as_u32(),
                module.to_u32(),
            ),
            Self::Private { module, name } => write!(
                f,
                "name (symbol {}) in module {} is private and cannot be imported",
                name.as_u32(),
                module.to_u32(),
            ),
            Self::ImportCycle { module, name } => write!(
                f,
                "import of name (symbol {}) forms a cycle at module {}",
                name.as_u32(),
                module.to_u32(),
            ),
        }
    }
}

impl core::error::Error for ResolveError {}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::string::ToString;

    use super::*;

    #[test]
    fn test_unknown_module_display_names_the_id() {
        let err = ResolveError::UnknownModule(ModuleId::from_index(4));
        assert!(err.to_string().contains('4'), "{err}");
    }

    #[test]
    fn test_duplicate_display_names_module_and_symbol() {
        let err = ResolveError::DuplicateName {
            module: ModuleId::from_index(2),
            name: Symbol::from_u32(7).expect("nonzero"),
        };
        let text = err.to_string();
        assert!(text.contains('2'), "{text}");
        assert!(text.contains('7'), "{text}");
    }

    #[test]
    fn test_error_is_clonable_and_equatable() {
        let a = ResolveError::Unresolved {
            module: ModuleId::from_index(1),
            name: Symbol::from_u32(1).expect("nonzero"),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }
}
