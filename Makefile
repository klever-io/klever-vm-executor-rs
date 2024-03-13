.PHONY: clean

TARGET :=
TARGET_DIR := ./target
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Linux)
	ifeq ($(PLATFORM),arm)
    	TARGET = --target aarch64-unknown-linux-gnu
		TARGET_DIR = ./target/aarch64-unknown-linux-gnu
	endif
endif
ifeq ($(UNAME_S),Darwin)
	ifeq ($(PLATFORM),arm)
		TARGET = --target aarch64-apple-darwin --target-dir ./target
		TARGET_DIR = ./target/aarch64-apple-darwin
	endif
endif

capi:
	cargo build ${TARGET} -p klever-chain-vm-executor-c-api --release

capi-linux-amd64: capi
	mv ${TARGET_DIR}/release/libklever_chain_vm_executor_c_api.so target/release/libvmexeccapi.so
	patchelf --set-soname libvmexeccapi.so target/release/libvmexeccapi.so

capi-linux-arm: capi
	mv ${TARGET_DIR}/release/libklever_chain_vm_executor_c_api.so target/release/libvmexeccapi_arm.so
	patchelf --set-soname libvmexeccapi_arm.so target/release/libvmexeccapi_arm.so

capi-osx-amd64: capi
	mv ${TARGET_DIR}/release/libklever_chain_vm_executor_c_api.dylib target/release/libvmexeccapi.dylib
	install_name_tool -id @rpath/libvmexeccapi.dylib target/release/libvmexeccapi.dylib

capi-osx-arm: capi
	mv ${TARGET_DIR}/release/libklever_chain_vm_executor_c_api.dylib target/release/libvmexeccapi_arm.dylib
	install_name_tool -id @rpath/libvmexeccapi_arm.dylib target/release/libvmexeccapi_arm.dylib

clean:
	cargo clean
	rm target/release/libvmexeccapi.so
	rm target/release/libvmexeccapi_arm.so
	rm target/release/libvmexeccapi.dylib
	rm target/release/libvmexeccapi_arm.dylib
	rm c-api/libvmexeccapi.h
