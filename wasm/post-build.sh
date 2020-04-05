#!/bin/bash
wasm-strip target/wasm32-unknown-unknown/release/wasm.wasm
cp target/wasm32-unknown-unknown/release/wasm.wasm ../public/helpers.wasm
