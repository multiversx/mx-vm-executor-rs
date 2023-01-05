.PHONY: clean

capi:
	cargo build -p mx-vm-executor-c-api --release

capi-linux-amd64: capi
	mv target/release/libmx_vm_executor_c_api.so target/release/libvmexeccapi.so
	patchelf --set-soname libvmexeccapi.so target/release/libvmexeccapi.so

capi-osx-amd64: capi
	mv target/release/libmx_vm_executor_c_api.dylib target/release/libvmexeccapi.dylib
	install_name_tool -id @executable_path/libvmexeccapi.dylib target/release/libvmexeccapi.dylib

clean:
	cargo clean
	rm target/release/libvmexeccapi.so
	rm target/release/libvmexeccapi.dylib
	rm exec-c-api/libvmexeccapi.h
