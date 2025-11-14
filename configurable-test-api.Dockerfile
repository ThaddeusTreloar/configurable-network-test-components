FROM rust:1.91.1-slim AS builder

RUN mkdir /build
WORKDIR /build

COPY . .

RUN cargo build --bin configurable-test-api --release

FROM rust:1.91.1-slim

RUN mkdir /app
WORKDIR /app

COPY --from=0 /build/target/release/configurable-test-api /app/configurable-test-api

CMD ["/app/configurable-test-api"]
