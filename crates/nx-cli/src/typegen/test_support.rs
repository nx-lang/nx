//! Helpers shared by the typegen test modules.

use std::fs;
use std::path::Path;

/// Writes a library directory with the given `(file name, source)` pairs.
pub(crate) fn write_library(root: &Path, files: &[(&str, &str)]) {
    fs::create_dir_all(root).expect("library dir");
    for (name, source) in files {
        fs::write(root.join(name), source).expect("library file");
    }
}
