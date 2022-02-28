FROM gitpod/workspace-full
ENV CARGO_HOME=/workspace/.cargo
RUN bash -cl "rustup toolchain install nightly"
RUN bash -cl "rustup toolchain add nightly-x86_64-unknown-linux-gnu"
RUN bash -cl "rustup default nightly"
#rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
