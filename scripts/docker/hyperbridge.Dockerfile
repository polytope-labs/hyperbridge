FROM docker.io/library/debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && update-ca-certificates

WORKDIR /

COPY ./target/release/hyperbridge ./


ENTRYPOINT ["./hyperbridge"]
