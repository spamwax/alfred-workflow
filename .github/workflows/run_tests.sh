#!/bin/bash

export RUST_TEST_NOCAPTURE=1

cargo test --features updater --lib -- --test-threads=1
# cargo test --features updater,ci --lib -- --test-threads=1

# cargo test --features updater --lib -- --ignored --test-threads=1
# cargo test --features updater,ci --lib -- --ignored --test-threads=1

# doc tests
cargo test --features updater --doc
# cargo test --features updater,ci --doc

