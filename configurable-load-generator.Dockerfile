FROM rust:1.91.1-slim AS builder

RUN mkdir /build
WORKDIR /build

COPY . .

RUN cargo build --bin configurable-load-generator --release

FROM rust:1.91.1-slim

RUN mkdir /app
WORKDIR /app

COPY --from=0 /build/target/release/configurable-load-generator /app/configurable-load-generator

CMD ["/app/configurable-load-generator"]
