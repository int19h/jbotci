FROM rust:1-bookworm AS build

ARG BASE_PATH=/
ARG EMBEDDING_BACKEND=transformers
ARG EMBEDDING_DTYPES="q4 q8"

WORKDIR /src

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates nodejs npm \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown
RUN cargo install dioxus-cli --version 0.7.9 --locked

COPY . .

RUN set -eux; \
    dtype_args=""; \
    for dtype in ${EMBEDDING_DTYPES}; do \
        dtype_args="${dtype_args} --embedding-dtype ${dtype}"; \
    done; \
    cargo xtask dist-server \
        --out-dir /opt/jbotci \
        --base-path "${BASE_PATH}" \
        --embedding-backend "${EMBEDDING_BACKEND}" \
        ${dtype_args}

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

EXPOSE 10000

USER jbotci

CMD ["./server"]
