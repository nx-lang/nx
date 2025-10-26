use std::path::PathBuf;

fn main() {
    let src_dir = PathBuf::from("tree-sitter-nx").join("src");

    // Tell cargo to rerun if the grammar changes
    println!("cargo:rerun-if-changed=grammar.js");
    println!("cargo:rerun-if-changed={}/parser.c", src_dir.display());
    println!("cargo:rerun-if-changed={}/scanner.c", src_dir.display());

    cc::Build::new()
        .include(&src_dir)
        .file(src_dir.join("parser.c"))
        .file(src_dir.join("scanner.c"))
        .compile("tree-sitter-nx");
}
