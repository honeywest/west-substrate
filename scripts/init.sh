#!/usr/bin/env bash

set -e

echo "*** Initialising WASM build environment"

if [ -z $CI_PROJECT_NAME ] ; then
   rustup update nightly
   rustup update nightly-2019-06-09
   rustup update stable
fi

rustup target add wasm32-unknown-unknown --toolchain nightly
rustup target add wasm32-unknown-unknown --toolchain nightly-2019-06-09
rustup default nightly-2019-06-09

# Install wasm-gc. It's useful for stripping slimming down wasm binaries.
command -v wasm-gc || \
	cargo +nightly install --git https://github.com/alexcrichton/wasm-gc --force
