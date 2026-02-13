FROM rust:1.91.1-slim AS builder

RUN mkdir /build
WORKDIR /build

COPY . .

RUN cargo build --release

FROM rust:1.91.1-slim

RUN mkdir /app
WORKDIR /app

COPY --from=0 /build/target/release/pingora-lb-test /app/pingora-lb-test

CMD ["/app/pingora-lb-test"]
