cd ./E2E
cargo clean
cargo run --release --features dhat-heap -- ..\..\NorthstarMods-1.30.0
cargo test --release --features timed -- --nocapture
copy .\dhat-heap.json ..\..\Profile\dhat-heap.json