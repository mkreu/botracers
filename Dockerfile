# syntax=docker/dockerfile:1.7

FROM rust:1-bookworm AS chef
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install --locked cargo-chef wasm-bindgen-cli
RUN rustup target add wasm32-unknown-unknown

FROM chef AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json -p racehub

COPY . .
RUN mkdir -p /out

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build -p racehub --release --locked \
    && ./scripts/build_web.sh --release \
    && cp /app/target/release/racehub /out/racehub

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 racehub \
    && useradd --uid 10001 --gid racehub --home /app --shell /usr/sbin/nologin racehub \
    && mkdir -p /data/racehub_artifacts /opt/racehub \
    && chown -R racehub:racehub /data /opt/racehub

COPY --from=builder /out/racehub /usr/local/bin/racehub
COPY --from=builder /app/web-dist /opt/racehub/web-dist

ENV RACEHUB_BIND=0.0.0.0:8787
ENV RACEHUB_DB_PATH=/data/racehub.db
ENV RACEHUB_ARTIFACTS_DIR=/data/racehub_artifacts
ENV RACEHUB_STATIC_DIR=/opt/racehub/web-dist
ENV RUST_LOG=info

EXPOSE 8787

USER racehub
ENTRYPOINT ["/usr/local/bin/racehub"]
