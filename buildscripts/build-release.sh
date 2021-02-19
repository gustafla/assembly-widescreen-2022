#!/usr/bin/env bash
set -e

# Work in this script's directory
cd "$(dirname $0)"

# Determine crate/binary name
crate=$(grep '^name[ \t]*=[ \t]*\".*\"' ../Cargo.toml | cut -d'=' -f 2 | tr -d '\t "')

for platform in host arm; do
    case ${platform} in
        host)
            strip="strip"
            ext=".$(uname -m)"
            ;;
        arm)
            target="arm-unknown-linux-gnueabihf"
            strip="arm-linux-gnueabihf-strip"
            ext=".arm"
            ;;
    esac

    # Prepare to build
    docker build -f "Dockerfile-${platform}" -t ${crate}-builder-${platform} .

    # Compile, link and strip
    docker run --rm -v "$(dirname $PWD)":/build -w /build ${crate}-builder-${platform}:latest sh -c "\
    cargo build ${target:+--target $target} --release && \
    $strip --strip-all target/$target/release/$crate && \
    chown -R $(id -u):$(id -g) target"

    # Build a final release directory
    mkdir -p out
    cp ../target/$target/release/$crate "out/${crate}${ext}"
done

# Remember to throw in music, sync data, shaders, etc
cp -r ../resources out
