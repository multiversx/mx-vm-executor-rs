.PHONY: capi capi-linux-amd64 capi-linux-arm capi-osx-amd64 capi-osx-arm \
	capi-osx-amd64-cross capi-linux-amd64-docker capi-linux-arm64-docker \
	capi-w7 capi-w7-linux-amd64 capi-w7-linux-arm capi-w7-osx-amd64 capi-w7-osx-arm \
	capi-w2 capi-w2-linux-amd64 capi-w2-linux-arm capi-w2-osx-amd64 capi-w2-osx-arm \
	ci-local clean check-macos

# cargo and docker already parallelise internally, and a parallel make would
# have two cargo invocations contend for the same target dir lock.
.NOTPARALLEL:

# Where `ci-local` gathers the artifacts, so the cross and container builds
# stay out of the host's target/release.
CI_LOCAL_DIR ?= target/ci-local

# Passed to the Docker builds as the `wasmer-fork` named build context. Only
# matters when vm-executor-wasmer/Cargo.toml has a `path = "../wasmer-fork/..."`
# override in place of the usual git dependency: the container can't see paths
# outside its build context, so the fork checkout has to be handed over
# explicitly. Defaults to an empty directory so the plain git-dependency case
# (no override) keeps working with no extra setup. Override with e.g.
# `make ci-local WASMER_FORK_DIR=../wasmer-fork` when testing local fork changes.
WASMER_FORK_DIR ?= Docker/empty-context

UNAME_S := $(shell uname -s)

capi:
	cargo build -p multiversx-chain-vm-executor-c-api-legacy --release

capi-linux-amd64: capi
	mv target/release/libmultiversx_chain_vm_executor_c_api_legacy.so target/release/libvmexeccapi.so
	patchelf --set-soname libvmexeccapi.so target/release/libvmexeccapi.so

capi-linux-arm: capi
	mv target/release/libmultiversx_chain_vm_executor_c_api_legacy.so target/release/libvmexeccapi_arm.so
	patchelf --set-soname libvmexeccapi_arm.so target/release/libvmexeccapi_arm.so

# A prerequisite, not an inline check in the recipe: under .NOTPARALLEL, make
# runs prerequisites in order and stops at the first failure, so listing this
# before `capi` below rejects a non-macOS host before the (potentially
# lengthy) cargo build starts, rather than after it.
check-macos:
	@test "$(UNAME_S)" = "Darwin" || { echo "This target requires a macOS host (produces a Mach-O .dylib)." >&2; exit 1; }

capi-osx-amd64: check-macos capi
	mv target/release/libmultiversx_chain_vm_executor_c_api_legacy.dylib target/release/libvmexeccapi.dylib
	install_name_tool -id @rpath/libvmexeccapi.dylib target/release/libvmexeccapi.dylib

capi-osx-arm: check-macos capi
	mv target/release/libmultiversx_chain_vm_executor_c_api_legacy.dylib target/release/libvmexeccapi_arm.dylib
	install_name_tool -id @rpath/libvmexeccapi_arm.dylib target/release/libvmexeccapi_arm.dylib

# --- The new C API ------------------------------------------------------------
# Built into separately named artifacts, so a consumer can link one, the other, or
# both: the two libraries share no exported symbols (`vm_exec_*` against
# `vm_exec2_*`), which is what makes running them side by side for differential
# testing possible.
#
# Not wired into `ci-local` or the release workflows yet. That happens when the Go
# VM starts consuming it, so the release assets do not grow a library nothing
# links against.

capi-w7:
	cargo build -p multiversx-chain-vm-executor-c-api-wasmer7 --release

capi-w7-linux-amd64: capi-w7
	mv target/release/libmultiversx_chain_vm_executor_c_api_wasmer7.so target/release/libvmexeccapi_w7.so
	patchelf --set-soname libvmexeccapi_w7.so target/release/libvmexeccapi_w7.so

capi-w7-linux-arm: capi-w7
	mv target/release/libmultiversx_chain_vm_executor_c_api_wasmer7.so target/release/libvmexeccapi_w7_arm.so
	patchelf --set-soname libvmexeccapi_w7_arm.so target/release/libvmexeccapi_w7_arm.so

capi-w7-osx-amd64: check-macos capi-w7
	mv target/release/libmultiversx_chain_vm_executor_c_api_wasmer7.dylib target/release/libvmexeccapi_w7.dylib
	install_name_tool -id @rpath/libvmexeccapi_w7.dylib target/release/libvmexeccapi_w7.dylib

capi-w7-osx-arm: check-macos capi-w7
	mv target/release/libmultiversx_chain_vm_executor_c_api_wasmer7.dylib target/release/libvmexeccapi_w7_arm.dylib
	install_name_tool -id @rpath/libvmexeccapi_w7_arm.dylib target/release/libvmexeccapi_w7_arm.dylib

# --- The new C API, Wasmer 2.2 ------------------------------------------------
# The same interface over the executor currently in production. Built alongside the
# Wasmer 7 library rather than instead of it: running the *same* runtime through the
# *new* boundary is what isolates the boundary as the variable, so a difference against
# the legacy `libvmexeccapi` is attributable to the refactor and not to a runtime change.
#
# Three libraries can therefore be linked at once — `vm_exec_*`, `vm_exec_w2_*` and
# `vm_exec_w7_*` share no exported symbols. On Linux that needs
# `-Wl,-Bsymbolic-functions`, because the two Wasmer versions do collide on their
# internal `wasmer_vm_*` symbols; macOS's two-level namespace handles it without help.

capi-w2:
	cargo build -p multiversx-chain-vm-executor-c-api-wasmer2 --release

capi-w2-linux-amd64: capi-w2
	mv target/release/libmultiversx_chain_vm_executor_c_api_wasmer2.so target/release/libvmexeccapi_w2.so
	patchelf --set-soname libvmexeccapi_w2.so target/release/libvmexeccapi_w2.so

capi-w2-linux-arm: capi-w2
	mv target/release/libmultiversx_chain_vm_executor_c_api_wasmer2.so target/release/libvmexeccapi_w2_arm.so
	patchelf --set-soname libvmexeccapi_w2_arm.so target/release/libvmexeccapi_w2_arm.so

capi-w2-osx-amd64: check-macos capi-w2
	mv target/release/libmultiversx_chain_vm_executor_c_api_wasmer2.dylib target/release/libvmexeccapi_w2.dylib
	install_name_tool -id @rpath/libvmexeccapi_w2.dylib target/release/libvmexeccapi_w2.dylib

capi-w2-osx-arm: check-macos capi-w2
	mv target/release/libmultiversx_chain_vm_executor_c_api_wasmer2.dylib target/release/libvmexeccapi_w2_arm.dylib
	install_name_tool -id @rpath/libvmexeccapi_w2_arm.dylib target/release/libvmexeccapi_w2_arm.dylib

# --- Local reproduction of the CI build matrix -------------------------------
# The four targets above are what CI runs natively, one per runner. The targets
# below drive the same builds from a single host: the two Linux libraries build
# in containers (any Docker host, cross-arch via buildx/QEMU), the way the
# libvmexeccapi-build-linux-arm64 workflow does. The two macOS libraries have no
# such option — Apple's terms don't allow macOS in a container, which is why CI
# itself uses real macos-15-intel/macos-latest runners rather than Docker for
# them — so they only run on a macOS host, and fail fast with an explanation
# everywhere else.

capi-osx-amd64-cross:
	@test "$(UNAME_S)" = "Darwin" || { \
	  echo "capi-osx-amd64-cross requires a macOS host: cross-compiling to Darwin" >&2; \
	  echo "needs Apple's linker/SDK, which Docker and a plain Linux toolchain don't" >&2; \
	  echo "have. Run this on a Mac, or rely on the CI runner." >&2; \
	  exit 1; \
	}
	rustup target add x86_64-apple-darwin
	cargo build -p multiversx-chain-vm-executor-c-api-legacy --release --target x86_64-apple-darwin
	cp target/x86_64-apple-darwin/release/libmultiversx_chain_vm_executor_c_api_legacy.dylib target/x86_64-apple-darwin/release/libvmexeccapi.dylib
	install_name_tool -id @rpath/libvmexeccapi.dylib target/x86_64-apple-darwin/release/libvmexeccapi.dylib

capi-linux-amd64-docker:
	docker buildx build --platform linux/amd64 --file Docker/linux.dockerfile \
	  --build-arg MAKE_TARGET=capi-linux-amd64 --build-arg ARTIFACT_NAME=libvmexeccapi.so \
	  --build-context wasmer-fork=$(WASMER_FORK_DIR) \
	  --tag mx-builder-amd64 --load .
	mkdir -p $(CI_LOCAL_DIR)
	docker run --platform linux/amd64 --rm mx-builder-amd64 cat /data/libvmexeccapi.so > $(CI_LOCAL_DIR)/libvmexeccapi.so

capi-linux-arm64-docker:
	docker buildx build --platform linux/arm64 --file Docker/linux.dockerfile \
	  --build-arg MAKE_TARGET=capi-linux-arm --build-arg ARTIFACT_NAME=libvmexeccapi_arm.so \
	  --build-context wasmer-fork=$(WASMER_FORK_DIR) \
	  --tag mx-builder-arm64 --load .
	mkdir -p $(CI_LOCAL_DIR)
	docker run --platform linux/arm64 --rm mx-builder-arm64 cat /data/libvmexeccapi_arm.so > $(CI_LOCAL_DIR)/libvmexeccapi_arm.so

ci-local: capi-linux-amd64-docker capi-linux-arm64-docker
ifeq ($(UNAME_S),Darwin)
ci-local: capi-osx-arm capi-osx-amd64-cross
endif
	mkdir -p $(CI_LOCAL_DIR)
	cp c-api/libvmexeccapi.h $(CI_LOCAL_DIR)/libvmexeccapi.h
	@if [ "$(UNAME_S)" = "Darwin" ]; then \
	  cp target/release/libvmexeccapi_arm.dylib $(CI_LOCAL_DIR)/libvmexeccapi_arm.dylib; \
	  cp target/x86_64-apple-darwin/release/libvmexeccapi.dylib $(CI_LOCAL_DIR)/libvmexeccapi.dylib; \
	else \
	  echo "--- Not on macOS: skipping libvmexeccapi.dylib / libvmexeccapi_arm.dylib" \
	       "(no container or cross-compile path exists for macOS; see README) ---"; \
	fi
	@echo "--- CI artifacts reproduced in $(CI_LOCAL_DIR) ---"
	@ls -1 $(CI_LOCAL_DIR)

clean:
	cargo clean
	rm -f target/release/libvmexeccapi.so
	rm -f target/release/libvmexeccapi_arm.so
	rm -f target/release/libvmexeccapi.dylib
	rm -f target/release/libvmexeccapi_arm.dylib
	rm -f c-api/libvmexeccapi.h
	rm -f target/release/libvmexeccapi_w7.so
	rm -f target/release/libvmexeccapi_w7_arm.so
	rm -f target/release/libvmexeccapi_w7.dylib
	rm -f target/release/libvmexeccapi_w7_arm.dylib
	rm -f c-api-wasmer7/libvmexeccapi_w7.h
	rm -f target/release/libvmexeccapi_w2.so
	rm -f target/release/libvmexeccapi_w2_arm.so
	rm -f target/release/libvmexeccapi_w2.dylib
	rm -f target/release/libvmexeccapi_w2_arm.dylib
	rm -f c-api-wasmer2/libvmexeccapi_w2.h
	rm -f target/x86_64-apple-darwin/release/libvmexeccapi.dylib
	rm -rf $(CI_LOCAL_DIR)
