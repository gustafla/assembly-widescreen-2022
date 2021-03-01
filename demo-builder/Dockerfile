FROM rust:slim-buster

# Install required debian packages
RUN apt-get update && \
    apt-get install --no-install-recommends -y \
    libgles2-mesa-dev libasound2-dev \
    wget git cmake pkg-config build-essential && \
    rm -rf /var/lib/apt/lists/*

# Install Rust arm toolchain
RUN rustup target add arm-unknown-linux-gnueabihf
RUN printf '[target.arm-unknown-linux-gnueabihf]\nlinker = "arm-linux-gnueabihf-gcc"' > $CARGO_HOME/config.toml

# Install the RPi GNU toolchain
RUN git clone --depth 1 https://github.com/raspberrypi/tools && \
cp -r tools/arm-bcm2708/arm-linux-gnueabihf/* /usr && rm -r /tools

# Build the RPi GPU driver userland
RUN git clone --depth 1 https://github.com/raspberrypi/userland && \
cd userland && \
./buildme && \
cd build/arm-linux/release && \
make install && \
rm -r /userland

# Build ALSA for ARM
RUN wget ftp://ftp.alsa-project.org/pub/lib/alsa-lib-1.1.8.tar.bz2 && \
tar xf alsa-lib-1.1.8.tar.bz2 && cd alsa-lib-1.1.8 && \
./configure --prefix /usr/arm-linux-gnueabihf --host arm-linux-gnueabihf && \
make -j $(nproc) && make install && rm -r /alsa-lib-1.1.8*
