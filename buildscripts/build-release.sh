#!/usr/bin/env bash
set -e

# Check that the CLI is being used at all
supported="host, arm"
[[ -z $@ ]] && echo Please choose at least one platform from \"$supported\" && exit 1

# Work in this script's directory
cd "$(dirname $0)"

# Determine crate/binary name
crate=$(grep '^name[ \t]*=[ \t]*\".*\"' ../Cargo.toml | cut -d'=' -f 2 | tr -d '\t "')

for platform in $@; do
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
        *)
            echo Supported plaforms are \"$supported\"
            exit 1
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
