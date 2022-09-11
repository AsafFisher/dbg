pip3 install --user -r requirements.txt
rustup update
rustup default nightly
rustup target add x86_64-unknown-none
rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu

cargo install cargo-binutils
rustup component add llvm-tools-preview
rustup component add clippy
rustup component add rustfmt
