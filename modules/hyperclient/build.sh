#!/usr/bin/env bash

rm -rf dist

pwd

wasm-pack build -t bundler -d dist/bundler --release --no-default-features --features=wasm
wasm-pack build -t nodejs -d dist/node --release --no-default-features --features=wasm
wasm-pack build -t web -d dist/web --release --no-default-features --features=wasm

rm dist/bundler/.gitignore dist/bundler/package.json dist/bundler/README.md dist/bundler/hyperclient.d.ts
rm dist/node/.gitignore dist/node/package.json dist/node/README.md dist/node/hyperclient.d.ts
rm dist/web/.gitignore dist/web/package.json dist/web/README.md dist/web/hyperclient.d.ts

cp hyperclient.d.ts dist/bundler
cp hyperclient.d.ts dist/node
cp hyperclient.d.ts dist/web
