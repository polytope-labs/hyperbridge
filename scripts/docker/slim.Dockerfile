FROM docker.io/library/debian:bullseye-slim

WORKDIR /

COPY ./hyperbridge ./


ENTRYPOINT ["./hyperbridge"]