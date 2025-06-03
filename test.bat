REM Switch to using workspace
cargo clean
cargo test 
cargo llvm-cov  --output-path ../Coverage/lcov.info
cargo llvm-cov report --html
REM This is the entire code coverage report, including E2E tests.
REM cargo run --features dhat-heap -- ..\..\northstar ..\..\northstar\.github\nativefuncs.json #For profiling