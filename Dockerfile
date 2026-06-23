# ENV S6_OVERLAY_VERSION=v3.2.2.0 around line 15; https://github.com/just-containers/s6-overlay/releases
FROM registry.fedoraproject.org/fedora:43 AS builder

WORKDIR /src

RUN dnf -y install cargo rust-src rust-std-static cargo-rpm-macros gcc \
gcc-c++ make openssl-devel pkgconf-pkg-config git \
&& dnf -y clean all \
&& rm -rf /var/cache/dnf

RUN cargo install worker-build --root /usr/local

COPY Cargo.toml ./
COPY miniflare-lease-client ./miniflare-lease-client
COPY src ./src

RUN cargo build --release --bins

FROM registry.fedoraproject.org/fedora:43

ARG S6_OVERLAY_VERSION=3.2.2.0
ARG TARGETARCH

ENV WORK_HOST=/work/host \
    PATH=/command:/usr/local/bin:/usr/bin:/bin

RUN set -eux; \
    dnf -y install \
      ca-certificates \
      curl \
      nodejs \
      npm \
      xz; \
    npm install -g wrangler miniflare; \
    dnf -y clean all; \
    rm -rf /var/cache/dnf /root/.npm

COPY --from=builder /src/target/release/worker-runtime-host-gen /usr/local/bin/worker-runtime-host-gen
COPY --from=builder /usr/local/bin/worker-build /usr/local/bin/worker-build
COPY --from=builder /src/target/release/worker-runtime-host-init /usr/local/bin/worker-runtime-host-init
COPY --from=builder /src/target/release/worker-runtime-host-docs /usr/local/bin/worker-runtime-host-docs
COPY --from=builder /src/target/release/worker-runtime-host-worker /usr/local/bin/worker-runtime-host-worker
COPY --from=builder /src/target/release/worker-runtime-host-watch /usr/local/bin/worker-runtime-host-watch

RUN set -eux; \
    arch="${TARGETARCH:-}"; \
    if [ -z "$arch" ]; then \
      arch="$(uname -m)"; \
    fi; \
    case "$arch" in \
      amd64|x86_64) s6_arch=x86_64 ;; \
      arm64|aarch64) s6_arch=aarch64 ;; \
      arm/v7|armv7|armv7l) s6_arch=arm ;; \
      arm/v6|armv6|armv6l|armhf) s6_arch=armhf ;; \
      386|i386|i686) s6_arch=i686 ;; \
      riscv64|s390x) s6_arch="$arch" ;; \
      *) echo "unsupported architecture: $arch" >&2; exit 1 ;; \
    esac; \
    curl -fsSL -o /tmp/s6-overlay-noarch.tar.xz \
      "https://github.com/just-containers/s6-overlay/releases/download/v${S6_OVERLAY_VERSION}/s6-overlay-noarch.tar.xz"; \
    tar -C / -Jxpf /tmp/s6-overlay-noarch.tar.xz; \
    curl -fsSL -o /tmp/s6-overlay-arch.tar.xz \
      "https://github.com/just-containers/s6-overlay/releases/download/v${S6_OVERLAY_VERSION}/s6-overlay-${s6_arch}.tar.xz"; \
    tar -C / -Jxpf /tmp/s6-overlay-arch.tar.xz; \
    rm -f /tmp/s6-overlay-noarch.tar.xz /tmp/s6-overlay-arch.tar.xz

RUN set -eux; \
    mkdir -p \
      /work/host \
      /etc/s6-overlay/s6-rc.d/user/contents.d \
      /etc/s6-overlay/s6-rc.d/worker-runtime-host-init/dependencies.d \
      /etc/s6-overlay/s6-rc.d/worker-runtime-host-docs/dependencies.d; \
    chmod 0755 /usr/local/bin/worker-runtime-host-gen; \
    chmod 0755 /usr/local/bin/worker-runtime-host-init; \
    chmod 0755 /usr/local/bin/worker-runtime-host-docs; \
    chmod 0755 /usr/local/bin/worker-runtime-host-worker; \
    chmod 0755 /usr/local/bin/worker-runtime-host-watch; \
    printf '%s\n' oneshot > /etc/s6-overlay/s6-rc.d/worker-runtime-host-init/type; \
    printf '%s\n' /usr/local/bin/worker-runtime-host-init > /etc/s6-overlay/s6-rc.d/worker-runtime-host-init/up; \
    : > /etc/s6-overlay/s6-rc.d/worker-runtime-host-init/dependencies.d/base; \
    printf '%s\n' longrun > /etc/s6-overlay/s6-rc.d/worker-runtime-host-docs/type; \
    printf '%s\n' '#!/bin/sh' 'exec /usr/local/bin/worker-runtime-host-docs' > /etc/s6-overlay/s6-rc.d/worker-runtime-host-docs/run; \
    chmod 0755 /etc/s6-overlay/s6-rc.d/worker-runtime-host-docs/run; \
    : > /etc/s6-overlay/s6-rc.d/worker-runtime-host-docs/dependencies.d/worker-runtime-host-init; \
    : > /etc/s6-overlay/s6-rc.d/user/contents.d/worker-runtime-host-docs

WORKDIR /work

ENTRYPOINT ["/init"]
