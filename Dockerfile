ARG ARCH=amd64

FROM rust:1.49-slim as base

WORKDIR /app
ADD . /app

FROM base as build-arm32v7
RUN apt-get update && apt-get install -y build-essential gcc-arm-linux-gnueabihf
RUN rustup target add armv7-unknown-linux-gnueabihf
RUN cargo build --release --target armv7-unknown-linux-gnueabihf
RUN mv /app/target/armv7-unknown-linux-gnueabihf/release/taribot /bin/taribot

FROM base as build-amd64
RUN cargo build --release
RUN mv /app/target/release/taribot /bin/taribot

FROM build-${ARCH} as final

FROM ${ARCH}/debian:buster-slim
COPY --from=final /bin/taribot /
CMD ["./taribot"]
