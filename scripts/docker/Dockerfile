FROM paritytech/ci-linux:production

COPY ./ ./

RUN RUSTFLAGS="-C link-args=-Wl,--allow-multiple-definition" cargo build --release -p hyperbridge --features goerli

ENTRYPOINT ["./target/release/hyperbridge"]
