FROM rust:1.67-alpine3.16 AS builder
RUN apk add openssl-dev musl-dev
COPY Cargo.toml Cargo.lock /
COPY src /src
# Flags to dynamically link to musl (avoids segfault in openssl)
RUN RUSTFLAGS='-C target-feature=-crt-static' cargo build --release

FROM alpine:3.16 AS app
RUN apk add openssl yt-dlp
COPY --from=builder /target/release/reelbot /reelbot
CMD ["/reelbot"]

