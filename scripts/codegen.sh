subxt codegen --url $1 | rustfmt --edition=2018 --emit=stdout | tee ./substrate/beefy/prover/src/runtime/$2.rs
