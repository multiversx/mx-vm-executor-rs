

.PHONY: capi
capi:
	cd exec-c-api && cargo build --release
	cd ..
	cp target/release/libelrond_capi.so libvmexeccapi.so
	cp libvmexeccapi.so /home/andreim/elrond/vm/wasm-vm/wasmer2
	cp exec-c-api/elrondcapi.h /home/andreim/elrond/vm/wasm-vm/wasmer2/libvmexeccapi.h

