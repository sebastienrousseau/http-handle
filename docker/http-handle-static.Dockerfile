# syntax=docker/dockerfile:1.7
FROM rust:1.88@sha256:4727898c104ecd2e22d780925832502faee9fe4e70581b8572af081370b315a0 as builder
WORKDIR /src
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-musl
COPY . .
RUN cargo build --release --bin http-handle --target x86_64-unknown-linux-musl

FROM gcr.io/distroless/static-debian12:nonroot@sha256:a9329520abc449e3b14d5bc3a6ffae065bdde0f02667fa10880c49b35c109fd1
WORKDIR /app
COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/http-handle /app/http-handle
ENV HTTP_HANDLE_ADDR=0.0.0.0:8080
EXPOSE 8080
ENTRYPOINT ["/app/http-handle"]
