FROM rust:1.90-bookworm AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY data ./data
COPY voices ./voices
RUN cargo build --locked --release --bin selmemd

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 10001 --create-home selmem \
    && mkdir -p /var/lib/selmem \
    && chown selmem:selmem /var/lib/selmem
COPY --from=builder /build/target/release/selmemd /usr/local/bin/selmemd
COPY --chmod=755 deploy/start.sh /usr/local/bin/start-selmem
USER selmem
WORKDIR /var/lib/selmem
ENV PORT=10000 SELMEM_PATH=/var/lib/selmem/demo.db SELMEM_NAME=Demo SELMEM_PROFILE=tender
EXPOSE 10000
ENTRYPOINT ["/usr/local/bin/start-selmem"]
