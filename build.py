import os

os.system("cargo run --bin generate_structs")
os.system('~/.cargo/bin/serdegen --language python3 --with-runtimes serde bincode --module-name structs --target-source-dir "python" output.yml')