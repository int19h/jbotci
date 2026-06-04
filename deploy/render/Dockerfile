# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS chef-base

WORKDIR /src

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates nodejs npm \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

RUN --mount=type=cache,id=jbotci-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=jbotci-cargo-git,target=/usr/local/cargo/git/db,sharing=locked \
    --mount=type=cache,id=jbotci-cargo-install-target,target=/tmp/cargo-install-target,sharing=locked \
    CARGO_TARGET_DIR=/tmp/cargo-install-target \
    cargo install dioxus-cli --version 0.7.9 --locked \
    && CARGO_TARGET_DIR=/tmp/cargo-install-target \
    cargo install cargo-chef --version 0.1.77 --locked

FROM chef-base AS planner

COPY . .

RUN cargo chef prepare --recipe-path recipe.json

FROM chef-base AS cacher

COPY --from=planner /src/recipe.json recipe.json

RUN --mount=type=cache,id=jbotci-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=jbotci-cargo-git,target=/usr/local/cargo/git/db,sharing=locked \
    cargo chef cook --locked --recipe-path recipe.json -p xtask --bin xtask

RUN --mount=type=cache,id=jbotci-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=jbotci-cargo-git,target=/usr/local/cargo/git/db,sharing=locked \
    cargo chef cook --locked --release --recipe-path recipe.json -p jbotci-server --bin jbotci-server

RUN --mount=type=cache,id=jbotci-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=jbotci-cargo-git,target=/usr/local/cargo/git/db,sharing=locked \
    cargo chef cook --locked --release --target wasm32-unknown-unknown --recipe-path recipe.json -p jbotci-web --bin jbotci-web

FROM cacher AS build

COPY . .

ARG BASE_PATH=/
ARG WEB_EMBEDDINGS_BASE_URL=https://assets.jbotci.app/embeddings/web/v1
ARG RENDER_GIT_COMMIT=
ARG JBOTCI_GIT_COMMIT=

RUN --mount=type=cache,id=jbotci-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=jbotci-cargo-git,target=/usr/local/cargo/git/db,sharing=locked \
    set -eux; \
    build_commit="${JBOTCI_GIT_COMMIT:-${RENDER_GIT_COMMIT:-}}"; \
    if ! printf '%s' "${build_commit}" | grep -Eq '^[0-9a-fA-F]{40}$'; then \
        echo >&2 "expected RENDER_GIT_COMMIT or JBOTCI_GIT_COMMIT build arg to be a 40-character hexadecimal Git commit hash"; \
        exit 1; \
    fi; \
    export JBOTCI_GIT_COMMIT="${build_commit}"; \
    export JBOTCI_WEB_EMBEDDINGS_BASE_URL="${WEB_EMBEDDINGS_BASE_URL}"; \
    cargo run -p xtask -- dist-server \
        --out-dir /opt/jbotci \
        --base-path "${BASE_PATH}" \
        --skip-web-embeddings; \
    rm -rf /src/target

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home /opt/jbotci --shell /usr/sbin/nologin jbotci

WORKDIR /opt/jbotci

COPY --from=build /opt/jbotci/server ./server
COPY --from=build /opt/jbotci/public ./public

ENV IP=0.0.0.0
ENV PORT=10000
ENV DIOXUS_ASSET_ROOT=/
ENV DIOXUS_PUBLIC_PATH=/opt/jbotci/public
ENV JBOTCI_WEB_EMBEDDINGS_BASE_URL=https://assets.jbotci.app/embeddings/web/v1

EXPOSE 10000

USER jbotci

CMD ["./server"]
