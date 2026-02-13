FROM rust:1.91.1-slim

RUN mkdir /app

WORKDIR /app

COPY ./target/release/configurable-test-api /app/configurable-test-api

CMD ["/app/configurable-test-api"]
