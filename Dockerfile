FROM rust:1.80-slim-bookworm as builder

WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

COPY src ./src
COPY assets ./assets
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/bot-rust /app/bot-rust
COPY --from=builder /usr/src/app/assets /app/assets

ENV PORT=8080
EXPOSE 8080

CMD ["/app/bot-rust"]
