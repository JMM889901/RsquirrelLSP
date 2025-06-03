#Switch to using workspace
cargo clean
cargo build --release
REM cargo test --release
copy .\target\release\LSP.exe .\Extension\RSqLSP\bin\LSP.exe
cd .\Extension\RSqLSP
call npm install
vsce package --allow-missing-repository --skip-license --out "..\..\..\Extension\RSqLSP.vsix"