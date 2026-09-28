.PHONY: capi capi-linux-amd64 capi-linux-arm capi-osx-amd64 capi-osx-arm \
	capi-osx-amd64-cross capi-linux-amd64-docker capi-linux-arm64-docker \
	ci-local clean check-macos

# cargo and docker already parallelise internally, and a parallel make would
# have two cargo invocations contend for the same target dir lock.
.NOTPARALLEL:

# Where `ci-local` gathers the artifacts, so the cross and container builds
# stay out of the host's target/release.
CI_LOCAL_DIR ?= target/ci-local

UNAME_S := $(shell uname -s)

capi:
	cargo build -p multiversx-chain-vm-executor-c-api --release

capi-linux-amd64: capi
	mv target/release/libmultiversx_chain_vm_executor_c_api.so target/release/libvmexeccapi.so
	patchelf --set-soname libvmexeccapi.so target/release/libvmexeccapi.so

capi-linux-arm: capi
	mv target/release/libmultiversx_chain_vm_executor_c_api.so target/release/libvmexeccapi_arm.so
	patchelf --set-soname libvmexeccapi_arm.so target/release/libvmexeccapi_arm.so

# A prerequisite, not an inline check in the recipe: under .NOTPARALLEL, make
# runs prerequisites in order and stops at the first failure, so listing this
# before `capi` below rejects a non-macOS host before the (potentially
# lengthy) cargo build starts, rather than after it.
check-macos:
	@test "$(UNAME_S)" = "Darwin" || { echo "This target requires a macOS host (produces a Mach-O .dylib)." >&2; exit 1; }

capi-osx-amd64: check-macos capi
	mv target/release/libmultiversx_chain_vm_executor_c_api.dylib target/release/libvmexeccapi.dylib
	install_name_tool -id @rpath/libvmexeccapi.dylib target/release/libvmexeccapi.dylib

capi-osx-arm: check-macos capi
	mv target/release/libmultiversx_chain_vm_executor_c_api.dylib target/release/libvmexeccapi_arm.dylib
	install_name_tool -id @rpath/libvmexeccapi_arm.dylib target/release/libvmexeccapi_arm.dylib

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
	cargo build -p multiversx-chain-vm-executor-c-api --release --target x86_64-apple-darwin
	cp target/x86_64-apple-darwin/release/libmultiversx_chain_vm_executor_c_api.dylib target/x86_64-apple-darwin/release/libvmexeccapi.dylib
	install_name_tool -id @rpath/libvmexeccapi.dylib target/x86_64-apple-darwin/release/libvmexeccapi.dylib

capi-linux-amd64-docker:
	docker buildx build --platform linux/amd64 --file Docker/linux.dockerfile \
	  --build-arg MAKE_TARGET=capi-linux-amd64 --build-arg ARTIFACT_NAME=libvmexeccapi.so \
	  --tag mx-builder-amd64 --load .
	mkdir -p $(CI_LOCAL_DIR)
	docker run --platform linux/amd64 --rm mx-builder-amd64 cat /data/libvmexeccapi.so > $(CI_LOCAL_DIR)/libvmexeccapi.so

capi-linux-arm64-docker:
	docker buildx build --platform linux/arm64 --file Docker/linux.dockerfile \
	  --build-arg MAKE_TARGET=capi-linux-arm --build-arg ARTIFACT_NAME=libvmexeccapi_arm.so \
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
	rm -f target/x86_64-apple-darwin/release/libvmexeccapi.dylib
	rm -rf $(CI_LOCAL_DIR)
