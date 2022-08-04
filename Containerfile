FROM docker.io/rust:slim-bullseye
RUN rustup target add aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu x86_64-pc-windows-gnu \
    && apt-get update && apt-get install -y build-essential g++ gcc-aarch64-linux-gnu gcc-mingw-w64-x86-64 g++-aarch64-linux-gnu g++-mingw-w64-x86-64 cmake pkg-config libasound2-dev libfontconfig-dev && rm -rf /var/lib/apt/lists/* \
    && printf '[target.aarch64-unknown-linux-gnu]\nlinker = "aarch64-linux-gnu-gcc"\n' \
    > /usr/local/cargo/config.toml
