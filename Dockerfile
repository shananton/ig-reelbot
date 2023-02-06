FROM rust:1.67-alpine3.16

RUN apk update
RUN apk add openssl openssl-dev musl-dev yt-dlp

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

CMD ["./target/release/reelbot"]

