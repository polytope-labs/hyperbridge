FROM docker.io/library/debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && update-ca-certificates

WORKDIR /

COPY ./target/release/governance-api ./

EXPOSE 7071

ENTRYPOINT ["./governance-api"]
