//! Helpers shared by the typegen test modules.

use std::fs;
use std::path::Path;

/// Writes a library directory with the given `(file name, source)` pairs.
pub(crate) fn write_library(root: &Path, files: &[(&str, &str)]) {
    fs::create_dir_all(root).expect("library dir");
    for (name, source) in files {
        let path = root.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("library subdirectory");
        }
        fs::write(&path, source).expect("library file");
    }
}
