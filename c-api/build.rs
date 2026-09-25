extern crate cbindgen;

use cbindgen::{Builder, Language, Style};
use std::{env, fs, path::PathBuf};

const HEADER_FILE_NAME: &str = "libvmexeccapi";

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut crate_wasmer_header_file = PathBuf::from(&crate_dir);
    crate_wasmer_header_file.push(HEADER_FILE_NAME);

    let out_dir = env::var("OUT_DIR").unwrap();
    let mut out_wasmer_header_file = PathBuf::from(&out_dir);
    out_wasmer_header_file.push(HEADER_FILE_NAME);

    // Generate the C bindings in the `OUT_DIR`.
    out_wasmer_header_file.set_extension("h");
    Builder::new()
        .with_crate(crate_dir)
        .with_language(Language::C)
        // Keep the historical formatting: no line wrapping, `typedef struct { ... } name;`
        // without a tag. Both differ from the cbindgen defaults and would otherwise rewrite
        // the whole header on every re-generation.
        .with_line_length(usize::MAX)
        .with_style(Style::Type)
        .generate()
        .expect("Unable to generate C bindings")
        .write_to_file(out_wasmer_header_file.as_path());

    // Copy the generated C bindings from `OUT_DIR` to
    // `CARGO_MANIFEST_DIR`.
    crate_wasmer_header_file.set_extension("h");
    out_wasmer_header_file.set_extension("h");
    fs::copy(
        out_wasmer_header_file.as_path(),
        crate_wasmer_header_file.as_path(),
    )
    .expect("Unable to copy the generated C bindings");
}
