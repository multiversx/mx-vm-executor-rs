# wasm-vm-executor-rs

VM wasmer 2.2 executor + specialized C API to be used from Go.

Call `make capi` in the root to get the binary and the C header.

## Reproducing the CI artifacts locally

CI builds one library per platform, each on its own runner. `make ci-local`
reproduces as many of the four as your host can produce, and collects them,
together with the generated header, in `target/ci-local`:

| Artifact | Built by | Needs |
| --- | --- | --- |
| `libvmexeccapi.so` | `linux/amd64` container (`capi-linux-amd64-docker`) | Docker |
| `libvmexeccapi_arm.so` | `linux/arm64` container (`capi-linux-arm64-docker`) | Docker |
| `libvmexeccapi_arm.dylib` | native (`capi-osx-arm`) | a macOS host |
| `libvmexeccapi.dylib` | cross-compiled to `x86_64-apple-darwin` (`capi-osx-amd64-cross`) | a macOS host |

The two container targets build under `Docker/linux.dockerfile`, the same file
the `libvmexeccapi-build-linux-arm64` workflow uses, parameterized by
`MAKE_TARGET`/`ARTIFACT_NAME`; the `rust` base image is multi-arch, so
`--platform` alone selects amd64 or arm64. They need a running Docker daemon
with buildx, and the non-native arch needs QEMU registered (Docker Desktop does
this automatically; on plain Docker Engine, run
`docker run --privileged --rm tonistiigi/binfmt --install all` once).

**The two macOS libraries can only be built on an actual macOS host.** There is
no Docker equivalent for them: Apple's terms don't allow macOS to run in a
container, which is exactly why CI itself uses real `macos-15-intel` /
`macos-latest` runners rather than Docker for those two matrix entries, and why
plain cross-compilation to `x86_64-apple-darwin` only works when the host
already has Apple's linker/SDK (i.e. is itself a Mac).

Running `make ci-local` on Linux (or any non-macOS host) therefore builds only
`libvmexeccapi.so` and `libvmexeccapi_arm.so`, skips the two `.dylib`s with an
explanatory message, and still succeeds. Invoking a macOS-only target directly
on such a host (`capi-osx-arm`, `capi-osx-amd64`, `capi-osx-amd64-cross`) fails
fast with an error saying so, rather than a cryptic `install_name_tool: command
not found`.

The Rust version is pinned in `rust-toolchain.toml`, the workflow files and
`Docker/linux.dockerfile`; all must be bumped together.
