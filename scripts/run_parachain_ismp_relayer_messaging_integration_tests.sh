#!/bin/bash


cargo test --package integration-test --lib -- --ignored submit_transfer_function_works -- --exact &&
cargo test --package integration-test --lib -- --ignored  get_request_works -- --exact
