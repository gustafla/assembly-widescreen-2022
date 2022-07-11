FROM docker.io/rust:slim-bullseye
RUN rustup target add aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu x86_64-pc-windows-gnu \
    && apt-get update && apt-get install -y gcc-aarch64-linux-gnu gcc-mingw-w64-x86-64 pkg-config libasound2-dev && rm -rf /var/lib/apt/lists/* \
    && printf '[target.aarch64-unknown-linux-gnu]\nlinker = "aarch64-linux-gnu-gcc"\n' \
    > /usr/local/cargo/config.toml
