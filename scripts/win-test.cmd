cd test-resources/external_crate
cargo build
cd ../..
cargo test --all-features -- --test-threads=1