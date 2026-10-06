use std::{
    env, fs,
    path::{Path, PathBuf},
};
fn collect(root: &Path, relative: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root.join(relative)).expect("starter source directory") {
        let entry = entry.expect("source entry");
        let kind = entry.file_type().expect("source type");
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.')
            || matches!(
                name.as_ref(),
                "target" | "node_modules" | "dist" | "artifacts"
            )
        {
            continue;
        }
        let path = relative.join(name.as_ref());
        if kind.is_dir() {
            collect(root, &path, files);
        } else if kind.is_file() {
            files.push(path);
        }
    }
}
fn main() {
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR").expect("manifest"))
        .join("../..")
        .canonicalize()
        .expect("root");
    let mut files = vec![];
    for dir in ["crates", "samples", "contracts", "dev", "docs", ".github"] {
        collect(&root, Path::new(dir), &mut files);
    }
    for file in [
        "Cargo.toml",
        "Cargo.lock",
        "LICENSE",
        "README.md",
        "AGENTS.md",
        "rust-toolchain.toml",
        ".gitignore",
        ".env.example",
        "Dockerfile",
        ".dockerignore",
    ] {
        if root.join(file).is_file() {
            files.push(file.into());
        }
    }
    files.sort();
    let mut source = "pub const FILES: &[(&str,&[u8])] = &[\n".to_string();
    for path in files {
        println!("cargo:rerun-if-changed={}", root.join(&path).display());
        source.push_str(&format!(
            "({:?},include_bytes!({:?})),\n",
            path.to_string_lossy(),
            root.join(&path).to_string_lossy()
        ));
    }
    source.push_str("];\n");
    fs::write(
        Path::new(&env::var("OUT_DIR").expect("out")).join("starter.rs"),
        source,
    )
    .expect("embedded starter");
}
