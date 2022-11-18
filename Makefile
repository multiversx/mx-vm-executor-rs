.PHONY: clean

capi:
	cargo build -p elrond-vm-exec-c-api --release

capi-linux-amd64: capi
	mv target/release/libelrond_vm_exec_c_api.so target/release/libvmexeccapi.so
	patchelf --set-soname libvmexeccapi.so target/release/libvmexeccapi.so

capi-osx-amd64: capi
	mv target/release/libelrond_vm_exec_c_api.dylib target/release/libvmexeccapi.dylib
	install_name_tool -id @executable_path/libvmexeccapi.dylib target/release/libvmexeccapi.dylib

capi-osx-amd64-d:
	cargo build -p elrond-vm-exec-c-api
	mv target/debug/libelrond_vm_exec_c_api.dylib target/debug/libvmexeccapi.dylib

	# copy libs and header [temporary hardcoded paths]
	sudo cp target/debug/libvmexeccapi.dylib /usr/local/lib
	cp target/debug/libvmexeccapi.dylib /Users/ovidiu/Documents/elrond/wasm-vm/wasmer2
	cp exec-c-api/vmexeccapi.h /Users/ovidiu/Documents/elrond/wasm-vm/wasmer2/libvmexeccapi.h

clean:
	cargo clean
	rm target/release/libvmexeccapi.so
	rm target/release/libvmexeccapi.dylib
	rm exec-c-api/libvmexeccapi.h
