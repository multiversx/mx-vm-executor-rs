

.PHONY: capi clean
capi:
	cargo build -p elrond-vm-exec-c-api --release
	cp target/release/libelrond_vm_exec_c_api.so libvmexeccapi.so
	cp libvmexeccapi.so /home/andreim/elrond/vm/wasm-vm/wasmer2
	cp exec-c-api/vmexeccapi.h /home/andreim/elrond/vm/wasm-vm/wasmer2/libvmexeccapi.h


capi-osx-amd64:
	cargo build -p elrond-vm-exec-c-api --release
	mv target/release/libelrond_vm_exec_c_api.dylib target/release/libvmexeccapi.dylib

	# copy libs and header [temporary hardcoded paths]
	sudo cp target/release/libvmexeccapi.dylib /usr/local/lib
	cp target/release/libvmexeccapi.dylib /Users/ovidiu/Documents/elrond/wasm-vm/wasmer2
	cp exec-c-api/vmexeccapi.h /Users/ovidiu/Documents/elrond/wasm-vm/wasmer2/libvmexeccapi.h

capi-osx-amd64-d:
	cargo build -p elrond-vm-exec-c-api
	mv target/debug/libelrond_vm_exec_c_api.dylib target/debug/libvmexeccapi.dylib

	# copy libs and header [temporary hardcoded paths]
	sudo cp target/debug/libvmexeccapi.dylib /usr/local/lib
	cp target/debug/libvmexeccapi.dylib /Users/ovidiu/Documents/elrond/wasm-vm/wasmer2
	cp exec-c-api/vmexeccapi.h /Users/ovidiu/Documents/elrond/wasm-vm/wasmer2/libvmexeccapi.h

clean:
	cargo clean
	rm libvmexeccapi.so
	rm exec-c-api/vmexeccapi.h

clean-osx-amd64:
	cargo clean

	# clean libs and header [temporary hardcoded paths]
	rm target/release/libvmexeccapi.dylib
	sudo rm /usr/local/lib/libvmexeccapi.dylib
	rm /Users/ovidiu/Documents/elrond/wasm-vm/wasmer2/libvmexeccapi.dylib
	rm /Users/ovidiu/Documents/elrond/wasm-vm/wasmer2/libvmexeccapi.h
	rm exec-c-api/vmexeccapi.h
