import os
from contextlib import contextmanager

# artifact dir pathlib
from pathlib import Path

# function that changes directory in a context manager
@contextmanager
def chdir(path):
    """
    Change directory in a context manager
    """
    prev_dir = os.getcwd()
    os.chdir(path)
    try:
        yield
    finally:
        os.chdir(prev_dir)

# rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
host_target = '$(rustc -vV | sed -n "s/^host: //p")'
target = "x86_64-unknown-none"
rust_target = f"./{target}.json"
with chdir('./exes/shellcode_linux'):
    os.environ['RUSTFLAGS'] = '-C relocation-model=pie -C link-arg=-nostartfiles -C link-arg=-nostdlib -C link-arg=-static -C link-arg=-T./shellcode.ld' # Used to be pie -C target-feature=+crt-static -L/usr/lib/x86_64-linux-musl
    os.system(f'cargo +nightly build --bin shellcode --release --verbose -Zbuild-std=core,alloc --target {rust_target}')  # 
artifact_path = Path(f'./target/{target}/release/shellcode')
CMD = f"objcopy {artifact_path.absolute()} /dev/null --dump-section .text=text.data"
os.system(CMD)
