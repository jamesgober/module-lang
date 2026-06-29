//! # module_lang
//!
//! Module and import resolution for multi-file languages.
//!
//! Scaffold release (v0.1.0). The public surface is being designed across the
//! 0.x series and frozen at v1.0. See `docs/API.md` and `dev/ROADMAP.md` for the
//! current phase scope.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    /// The scaffold carries no public surface yet (see `dev/ROADMAP.md`); this
    /// test exists so the crate compiles and links under the test harness, and
    /// so CI has a unit test to run before the resolution code lands.
    #[test]
    fn crate_builds_under_test_harness() {}
}
