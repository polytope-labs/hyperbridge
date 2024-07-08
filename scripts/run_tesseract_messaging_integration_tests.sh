#!/bin/bash


cargo test --package integration-test --lib -- --ignored parachain_messaging -- --exact &&
cargo test --package integration-test --lib -- --ignored get_request_works -- --exact
