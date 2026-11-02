extern crate cbindgen;

use cbindgen::{Builder, Config, Language, Style};
use std::{env, fs, path::PathBuf};

const HEADER_FILE_NAME: &str = "libvmexeccapi_common";

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut crate_header_file = PathBuf::from(&crate_dir);
    crate_header_file.push(HEADER_FILE_NAME);

    let out_dir = env::var("OUT_DIR").unwrap();
    let mut out_header_file = PathBuf::from(&out_dir);
    out_header_file.push(HEADER_FILE_NAME);

    // Generate the C bindings in the `OUT_DIR`.
    out_header_file.set_extension("h");
    // Reads `cbindgen.toml`, which force-exports the types no exported function mentions.
    let config = Config::from_root_or_default(&crate_dir);

    Builder::new()
        .with_config(config)
        .with_crate(&crate_dir)
        .with_language(Language::C)
        // Same formatting choices as the legacy `c-api` header, so the two are easy to
        // diff against each other.
        .with_line_length(usize::MAX)
        .with_style(Style::Both)
        .generate()
        .expect("Unable to generate C bindings")
        .write_to_file(out_header_file.as_path());

    // Copy the generated C bindings from `OUT_DIR` to `CARGO_MANIFEST_DIR`.
    crate_header_file.set_extension("h");
    out_header_file.set_extension("h");
    fs::copy(out_header_file.as_path(), crate_header_file.as_path())
        .expect("Unable to copy the generated C bindings");
}
