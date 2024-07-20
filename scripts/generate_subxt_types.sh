#!/bin/bash

subxt codegen --derive=PartialEq --derive=Eq --derive=Clone  --url=$1 | rustfmt --edition=2018 --emit=stdout | tee ./modules/utils/subxt/src/$2.rs
