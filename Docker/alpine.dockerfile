FROM rust:1.88-alpine

RUN apk add --no-cache \
    musl-dev \
    git \
    build-base \
    openssl-dev \
    make \
    patchelf

ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

COPY . /repository
WORKDIR /repository

# Force native compilation (Alpine uses musl natively, but we need to allow cdylib)
ENV RUSTFLAGS="-C target-feature=-crt-static"

RUN make capi

# Detect architecture, rename, and set soname (consistent with glibc builds)
RUN ARCH=$(uname -m) && \
    mkdir -p /data && \
    if [ "$ARCH" = "x86_64" ]; then \
        mv /repository/target/release/libklever_chain_vm_executor_c_api.so /data/libvmexeccapi-musl.so && \
        patchelf --set-soname libvmexeccapi.so /data/libvmexeccapi-musl.so; \
    elif [ "$ARCH" = "aarch64" ]; then \
        mv /repository/target/release/libklever_chain_vm_executor_c_api.so /data/libvmexeccapi_arm-musl.so && \
        patchelf --set-soname libvmexeccapi_arm.so /data/libvmexeccapi_arm-musl.so; \
    else \
        echo "Unsupported architecture: $ARCH" && exit 1; \
    fi
