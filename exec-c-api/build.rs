extern crate cbindgen;

use cbindgen::{Builder, Language};
use std::{env, fs, path::PathBuf};

const HEADER_FILE_NAME: &str = "vmexeccapi";

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut crate_wasmer_header_file = PathBuf::from(&crate_dir);
    crate_wasmer_header_file.push(HEADER_FILE_NAME);

    let out_dir = env::var("OUT_DIR").unwrap();
    let mut out_wasmer_header_file = PathBuf::from(&out_dir);
    out_wasmer_header_file.push(HEADER_FILE_NAME);

    let pre_header = r#"
#if !defined(WASMER_H_MACROS)

#define WASMER_H_MACROS

// Define the `ARCH_X86_X64` constant.
#if defined(MSVC) && defined(_M_AMD64)
#  define ARCH_X86_64
#elif (defined(GCC) || defined(__GNUC__) || defined(__clang__)) && defined(__x86_64__)
#  define ARCH_X86_64
#endif

// Compatibility with non-Clang compilers.
#if !defined(__has_attribute)
#  define __has_attribute(x) 0
#endif

// Compatibility with non-Clang compilers.
#if !defined(__has_declspec_attribute)
#  define __has_declspec_attribute(x) 0
#endif

// Define the `DEPRECATED` macro.
#if defined(GCC) || defined(__GNUC__) || __has_attribute(deprecated)
#  define DEPRECATED(message) __attribute__((deprecated(message)))
#elif defined(MSVC) || __has_declspec_attribute(deprecated)
#  define DEPRECATED(message) __declspec(deprecated(message))
#endif

#endif // WASMER_H_MACROS
"#
    .to_string();

    // Generate the C bindings in the `OUT_DIR`.
    out_wasmer_header_file.set_extension("h");
    Builder::new()
        .with_crate(crate_dir.clone())
        .with_language(Language::C)
        .with_header(&pre_header)
        .with_define("target_family", "windows", "_WIN32")
        .with_define("target_arch", "x86_64", "ARCH_X86_64")
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
