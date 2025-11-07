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


docker-build-alpine: docker-build-alpine-amd64 docker-build-alpine-arm64

docker-build-alpine-amd64:
	docker buildx build --platform linux/amd64 -t libvm_alpine_amd64 -f Docker/alpine.dockerfile .
	mkdir -p output
	docker run --platform linux/amd64 --rm -v $(shell pwd)/output:/output libvm_alpine_amd64 sh -c "cp /data/libvmexeccapi-musl.so /output/"

docker-build-alpine-arm64:
	docker buildx build --platform linux/arm64 -t libvm_alpine_arm64 -f Docker/alpine.dockerfile .
	mkdir -p output
	docker run --platform linux/arm64 --rm -v $(shell pwd)/output:/output libvm_alpine_arm64 sh -c "cp /data/libvmexeccapi_arm-musl.so /output/"

clean:
	cargo clean
	rm -f target/release/libvmexeccapi.so
	rm -f target/release/libvmexeccapi_arm.so
	rm -f target/release/libvmexeccapi.dylib
	rm -f target/release/libvmexeccapi_arm.dylib
	rm -f target/release/libvmexeccapi-musl.so
	rm -f target/release/libvmexeccapi_arm-musl.so
	rm -f c-api/libvmexeccapi.h
	rm -rf output/

docker-build-arm:
	docker buildx build --platform linux/arm64/v8 -t libvm_arm -f Docker/arm64.dockerfile .
