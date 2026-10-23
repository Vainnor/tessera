# The sync server image: a static-ish release build on distroless, running as nonroot on :8787.
# Scaffolding: the server is a skeleton today. Build from the repo root:
#   docker build -f crates/server/Dockerfile .

FROM rust:1.98.1-slim-bookworm@sha256:ff521445a372125ed4f76e1453a1f8098f2d05332d1601d30db1c1f62757e730 AS build
WORKDIR /src
# The image already holds the toolchain rust-toolchain.toml pins; this stops rustup from fetching
# the clippy and rustfmt components the file also lists.
ENV RUSTUP_TOOLCHAIN=1.98.1 CARGO_TERM_COLOR=never
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p tessera-server \
    && cp target/release/tessera-server /tessera-server

FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f
COPY --from=build /tessera-server /usr/local/bin/tessera-server
COPY crates/server/container.toml /etc/tessera/server.toml
ENV TESSERA_SERVER_CONFIG=/etc/tessera/server.toml
EXPOSE 8787
ENTRYPOINT ["/usr/local/bin/tessera-server"]
