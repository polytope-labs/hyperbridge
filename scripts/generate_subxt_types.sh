#!/bin/bash

subxt codegen --derive=PartialEq --derive=Eq --derive=Clone  --url=$1 | rustfmt --edition=2018 --emit=stdout | tee ./modules/subxt/utils/src/$2.rs
