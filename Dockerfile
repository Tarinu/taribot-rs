FROM rust:1.45-slim as build

WORKDIR /app
ADD . /app
RUN cargo build --release

FROM gcr.io/distroless/cc
COPY --from=build /app/target/release/taribot /
CMD ["./taribot"]
