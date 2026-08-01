FROM rust:1.89-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src
COPY tests ./tests
RUN cargo build --release --locked

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl openssl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/replay-control /usr/local/bin/replay-control
COPY docker/entrypoint.sh /usr/local/bin/replay-entrypoint

RUN chmod 0755 /usr/local/bin/replay-control /usr/local/bin/replay-entrypoint \
    && mkdir -p /var/lib/replay-control /run/replay-secrets \
    && chmod 0700 /var/lib/replay-control /run/replay-secrets

EXPOSE 8080/tcp
ENTRYPOINT ["/usr/local/bin/replay-entrypoint"]
CMD ["broker"]
