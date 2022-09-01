

.PHONY: capi clean
capi:
	cd exec-c-api && cargo build --release
	cd ..
	cp target/release/libelrond_vm_exec_c_api.so libvmexeccapi.so
	cp libvmexeccapi.so /home/andreim/elrond/vm/wasm-vm/wasmer2
	cp exec-c-api/vmexeccapi.h /home/andreim/elrond/vm/wasm-vm/wasmer2/libvmexeccapi.h

clean:
	cargo clean
	rm libvmexeccapi.so
	rm exec-c-api/vmexeccapi.h

