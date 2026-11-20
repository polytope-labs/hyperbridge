FROM docker.io/library/debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && update-ca-certificates

WORKDIR /omniplex

COPY ./target/release/omniplex /usr/local/bin/omniplex
COPY ./omniplex/config ./config

ENV INDEXER_RUN_DIR=/omniplex/run

ENTRYPOINT ["omniplex"]
