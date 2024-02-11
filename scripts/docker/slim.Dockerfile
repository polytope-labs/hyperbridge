FROM docker.io/library/debian:bullseye-slim

RUN apt-get update && apt-get install -y ca-certificates && update-ca-certificates

WORKDIR /

COPY ./hyperbridge ./


ENTRYPOINT ["./hyperbridge"]