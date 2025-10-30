//! Salsa database for incremental computation.
//!
//! This module defines the query groups and database implementation for
//! incremental parsing, lowering, and semantic analysis.

use crate::{lower, Module, SourceId};
use nx_syntax::parse_str;
use std::sync::Arc;

/// The main query group for NX language analysis.
///
/// This defines all the incremental queries that Salsa will manage.
/// Queries are automatically cached and invalidated when their inputs change.
#[salsa::query_group(NxDatabaseStorage)]
pub trait NxDatabase: salsa::Database {
    /// Input query: Source text for a file.
    ///
    /// This is set manually via `set_source_text()`. When it changes,
    /// all derived queries that depend on it are automatically invalidated.
    ///
    /// # Example
    ///
    /// ```ignore
    /// db.set_source_text(file_id, Arc::new("let x = 42;".to_string()));
    /// let text = db.source_text(file_id);
    /// ```
    #[salsa::input]
    fn source_text(&self, file: SourceId) -> Arc<String>;

    /// Input query: File name for a source file.
    ///
    /// Used for error reporting and diagnostics.
    #[salsa::input]
    fn file_name(&self, file: SourceId) -> Arc<String>;

    /// Derived query: Lower source directly to HIR.
    ///
    /// This query is automatically cached. When `source_text(file)` changes,
    /// this query is re-executed. Otherwise, the cached result is returned.
    ///
    /// Note: This query performs both parsing and lowering in one step.
    /// This avoids storing the tree-sitter Tree (which doesn't implement Eq)
    /// in Salsa's cache.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let module = db.lower_to_hir(file_id);
    /// ```
    fn lower_to_hir(&self, file: SourceId) -> Arc<Module>;
}

/// Implementation of the lower_to_hir query.
///
/// This is called automatically by Salsa when `db.lower_to_hir(file)` is invoked
/// and the result is not cached.
///
/// This function performs both parsing and lowering in one step, which allows
/// us to avoid storing the tree-sitter Tree in Salsa's cache (it doesn't implement Eq).
fn lower_to_hir(db: &dyn NxDatabase, file: SourceId) -> Arc<Module> {
    let source = db.source_text(file);
    let file_name = db.file_name(file);

    // Parse the source
    let parse_result = parse_str(&source, &file_name);

    if let Some(ref tree) = parse_result.tree {
        let root = tree.root();
        let module = lower(root, file);
        Arc::new(module)
    } else {
        // If parsing failed, return an empty module
        Arc::new(Module::new(file))
    }
}

/// The default database implementation.
///
/// This is a simple in-memory database that stores all data in RAM.
/// For production use, you might want a more sophisticated implementation
/// with persistent storage or memory limits.
///
/// # Example
///
/// ```
/// use nx_hir::db::{NxDatabase, DatabaseImpl};
/// use nx_hir::SourceId;
/// use std::sync::Arc;
///
/// let mut db = DatabaseImpl::default();
/// let file = SourceId::new(0);
///
/// // Set source text (input query)
/// db.set_source_text(file, Arc::new("let x = 42;".to_string()));
/// db.set_file_name(file, Arc::new("example.nx".to_string()));
///
/// // Lower to HIR (derived query - automatically cached)
/// let module = db.lower_to_hir(file);
/// assert_eq!(module.items().len(), 0); // No top-level items in "let x = 42;"
/// ```
#[salsa::database(NxDatabaseStorage)]
#[derive(Default)]
pub struct DatabaseImpl {
    storage: salsa::Storage<Self>,
}

impl salsa::Database for DatabaseImpl {}

impl salsa::ParallelDatabase for DatabaseImpl {
    fn snapshot(&self) -> salsa::Snapshot<Self> {
        salsa::Snapshot::new(DatabaseImpl {
            storage: self.storage.snapshot(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = DatabaseImpl::default();
        // Database is created successfully
        drop(db);
    }

    #[test]
    fn test_source_text_query() {
        let mut db = DatabaseImpl::default();
        let file = SourceId::new(0);

        let source = Arc::new("let x = 42;".to_string());
        db.set_source_text(file, source.clone());

        assert_eq!(db.source_text(file), source);
    }

    #[test]
    fn test_file_name_query() {
        let mut db = DatabaseImpl::default();
        let file = SourceId::new(0);

        let name = Arc::new("example.nx".to_string());
        db.set_file_name(file, name.clone());

        assert_eq!(db.file_name(file), name);
    }

    #[test]
    fn test_lower_query() {
        let mut db = DatabaseImpl::default();
        let file = SourceId::new(0);

        db.set_source_text(file, Arc::new("let x = 42;".to_string()));
        db.set_file_name(file, Arc::new("test.nx".to_string()));

        let module = db.lower_to_hir(file);
        assert_eq!(module.source_id, file);
    }

    #[test]
    fn test_lower_caching() {
        let mut db = DatabaseImpl::default();
        let file = SourceId::new(0);

        db.set_source_text(file, Arc::new("let x = 42;".to_string()));
        db.set_file_name(file, Arc::new("test.nx".to_string()));

        let module1 = db.lower_to_hir(file);
        let module2 = db.lower_to_hir(file);

        // Arc pointers should be the same (cached)
        assert!(Arc::ptr_eq(&module1, &module2));
    }

    #[test]
    fn test_incremental_update() {
        let mut db = DatabaseImpl::default();
        let file1 = SourceId::new(0);
        let file2 = SourceId::new(1);

        // Set up two files
        db.set_source_text(file1, Arc::new("let x = 1;".to_string()));
        db.set_file_name(file1, Arc::new("file1.nx".to_string()));

        db.set_source_text(file2, Arc::new("let y = 2;".to_string()));
        db.set_file_name(file2, Arc::new("file2.nx".to_string()));

        // Process both files
        let module1_v1 = db.lower_to_hir(file1);
        let module2_v1 = db.lower_to_hir(file2);

        // Update file1 only
        db.set_source_text(file1, Arc::new("let x = 99;".to_string()));

        let module1_v2 = db.lower_to_hir(file1);
        let module2_v2 = db.lower_to_hir(file2);

        // file1 should be recomputed
        assert!(!Arc::ptr_eq(&module1_v1, &module1_v2));

        // file2 should be cached (unchanged)
        assert!(Arc::ptr_eq(&module2_v1, &module2_v2));
    }
}
