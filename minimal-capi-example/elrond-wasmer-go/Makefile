ROOT_DIR := $(dir $(realpath $(lastword $(MAKEFILE_LIST))))

# PHONY means that it doesn't correspond to a file; it always runs the build commands.

.PHONY: build
build:
	cd ../elrond-wasmer-lib && cargo build --release
	cp ../elrond-wasmer-lib/target/release/libelrondwasmer.so libcapiprototype.so
	go build

.PHONY: run
run:
	RUST_LOG=trace ./elrond-wasmer-go

.PHONY: clean
clean:
	rm -rf elrond-wasmer-go libelrondwasmer.so
