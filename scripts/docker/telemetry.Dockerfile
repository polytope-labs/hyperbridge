FROM docker.io/library/debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates gcc-4.9 libstdc++6 && update-ca-certificates

WORKDIR /

COPY ./target/release/telemetry-server ./

EXPOSE 3000

ENTRYPOINT ["./telemetry-server"]