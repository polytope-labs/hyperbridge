FROM docker.io/library/debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates librocksdb7.8 libsnappy-dev && \
    update-ca-certificates && \
    ln -s /usr/lib/x86_64-linux-gnu/librocksdb.so.7.8 /usr/lib/x86_64-linux-gnu/librocksdb.so.6.11 && \
    ldconfig && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /

COPY ./target/release/polyhedron ./

ENTRYPOINT ["./polyhedron"]
