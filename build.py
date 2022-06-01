import click
import os
from contextlib import contextmanager

# artifact dir pathlib
from pathlib import Path
target_archs = ["aarch", "x86_64"]

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


def run_build(arch):
    os.environ['RUSTFLAGS'] = ""
    # rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
    host_target = '$(rustc -vV | sed -n "s/^host: //p")'
    target = f"{arch}-unknown-none"
    
    if "x86_64" in target:
        os.environ['RUSTFLAGS'] = "-C link-arg=-nostartfiles "
    rust_target = f"./{target}.json"
    with chdir('./exes/shellcode_linux'):
        os.environ['RUSTFLAGS'] += '-C relocation-model=pie -C link-arg=-nostdlib -C link-arg=-static -C link-arg=-T./shellcode.ld' # Used to be pie -C target-feature=+crt-static -L/usr/lib/x86_64-linux-musl
        os.system(f'cargo +nightly build --bin shellcode --release --verbose -Zbuild-std=core,alloc --target {rust_target}')  # 
    artifact_path = Path(f'./target/{target}/release/shellcode')
    CMD = f"rust-objcopy {artifact_path.absolute()} /dev/null --dump-section .text=text.data"
    os.system(CMD)

@click.command()
@click.argument("arch", type=click.Choice(target_archs))
def build(arch):
    run_build(arch)

if __name__ == "__main__":
    build()
