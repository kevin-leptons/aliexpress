# NOTES:
# * Unable to cache both testing and building

FROM rust:1.64-slim as builder
WORKDIR /package
RUN apt update && \
    apt install -y curl && \
    rustup component add rustfmt
COPY Cargo.toml Cargo.toml
RUN echo "fn main() {}" > dummy.rs && \
    sed -i 's/src\/main.rs/dummy.rs/' Cargo.toml && \
    cargo build --release
COPY . .
RUN cargo fmt --check && \
    cargo build --release

FROM ubuntu:22.10
WORKDIR /package
COPY --from=builder /package/target/release/dropshipping_data start
RUN chmod +x start
ENTRYPOINT ["/package/start"]
