FROM docker.io/library/debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates librocksdb-dev rocksdb-tools libsnappy-dev && update-ca-certificates

RUN ldconfig

WORKDIR /

COPY ./target/release/polyhedron ./

ENTRYPOINT ["./polyhedron"]
