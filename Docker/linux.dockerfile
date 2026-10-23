# Builds libvmexeccapi for Linux, amd64 or arm64. The caller picks the arch via
# `docker buildx build --platform`; the official `rust` image is multi-arch, so
# the same tag resolves to the right base image on either platform.
FROM rust:1.98

# ARTIFACT_NAME must match what MAKE_TARGET produces in target/release (see Makefile).
ARG MAKE_TARGET=capi-linux-amd64
ARG ARTIFACT_NAME=libvmexeccapi.so

# wget and git already ship in the base image; only these two are new.
RUN apt-get update && apt-get install -y --no-install-recommends \
    patchelf \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

# `wasmer-fork` is a named build context (see Makefile: WASMER_FORK_DIR). It
# defaults to an empty directory, and is only populated when a `path =
# "../wasmer-fork/..."` override is in use in vm-executor-wasmer/Cargo.toml —
# the sibling layout here mirrors that of the local checkout so those
# relative paths resolve the same way on the host and in the container.
COPY . /repository/mx-vm-executor-rs
COPY --from=wasmer-fork . /repository/wasmer-fork
WORKDIR /repository/mx-vm-executor-rs
RUN make ${MAKE_TARGET}
RUN mkdir /data && cp /repository/mx-vm-executor-rs/target/release/${ARTIFACT_NAME} /data/${ARTIFACT_NAME}
