FROM docker.io/library/debian:bullseye-slim

WORKDIR /

COPY ./target/release/hyperbridge ./


ENTRYPOINT ["./hyperbridge"]