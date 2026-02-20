# syntax=docker/dockerfile:1.7
FROM rust:1.87 as builder
WORKDIR /src
COPY . .
RUN cargo build --release --bin http-handle

FROM gcr.io/distroless/cc-debian12:nonroot@sha256:7e5b8df2f4d36f5599ef4ab856d7d444922531709becb03f3368c6d797d0a5eb
WORKDIR /app
COPY --from=builder /src/target/release/http-handle /app/http-handle
ENV HTTP_HANDLE_ADDR=0.0.0.0:8080
EXPOSE 8080
ENTRYPOINT ["/app/http-handle"]
