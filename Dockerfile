# syntax = docker/dockerfile:experimental

FROM rust:1.51-slim as build

WORKDIR /app
ADD . /app

RUN cargo build --release

FROM debian:buster-slim
COPY --from=build /app/target/release/taribot /
CMD ["./taribot"]
