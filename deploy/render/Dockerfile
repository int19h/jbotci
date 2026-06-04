FROM rust:1-bookworm AS build

ARG BASE_PATH=/
ARG WEB_EMBEDDINGS_BASE_URL=https://assets.jbotci.app/embeddings/web/v1

WORKDIR /src

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates nodejs npm \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown
RUN cargo install dioxus-cli --version 0.7.9 --locked

COPY . .

RUN set -eux; \
    export JBOTCI_WEB_EMBEDDINGS_BASE_URL="${WEB_EMBEDDINGS_BASE_URL}"; \
    cargo run -p xtask -- dist-server \
        --out-dir /opt/jbotci \
        --base-path "${BASE_PATH}" \
        --skip-web-embeddings

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
