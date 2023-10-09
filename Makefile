.PHONY: clean

capi:
	cargo build -p multiversx-chain-vm-executor-c-api --release

capi-linux-amd64: capi
	mv target/release/libmultiversx_chain_vm_executor_c_api.so target/release/libvmexeccapi.so
	patchelf --set-soname libvmexeccapi.so target/release/libvmexeccapi.so

capi-linux-arm: capi
	mv target/release/libmultiversx_chain_vm_executor_c_api.so target/release/libvmexeccapi_arm.so
	patchelf --set-soname libvmexeccapi_arm.so target/release/libvmexeccapi_arm.so

capi-osx-amd64: capi
	mv target/release/libmultiversx_chain_vm_executor_c_api.dylib target/release/libvmexeccapi.dylib
	install_name_tool -id @rpath/libvmexeccapi.dylib target/release/libvmexeccapi.dylib

capi-osx-arm: capi
	mv target/release/libmultiversx_chain_vm_executor_c_api.dylib target/release/libvmexeccapi_arm.dylib
	install_name_tool -id @rpath/libvmexeccapi_arm.dylib target/release/libvmexeccapi_arm.dylib

clean:
	cargo clean
	rm target/release/libvmexeccapi.so
	rm target/release/libvmexeccapi_arm.so
	rm target/release/libvmexeccapi.dylib
	rm target/release/libvmexeccapi_arm.dylib
	rm c-api/libvmexeccapi.h
