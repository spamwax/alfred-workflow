#!/usr/bin/env bash

export RUST_TEST_NOCAPTURE=1
if [ -n "$CARGO_REGISTRY_TOKEN" ]; then
  echo "${CARGO_REGISTRY_TOKEN:1:3}"
else
  echo "didn't get CARGO_REGISTRY_TOKEN!"
fi
exit

cargo test --features updater --lib -- --test-threads=1
# cargo test --features updater,ci --lib -- --test-threads=1

# cargo test --features updater --lib -- --ignored --test-threads=1
# cargo test --features updater,ci --lib -- --ignored --test-threads=1

# doc tests
cargo test --features updater --doc
# cargo test --features updater,ci --doc

