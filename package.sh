#!/usr/bin/env bash
set -e

# Check that the CLI is being used at all
supported="host, rpi"
[[ -z $@ ]] && echo Please choose at least one platform from \"$supported\" && exit 1

# Work in this script's directory
cd "$(dirname $0)"

# Remove previous build(s)
rm -rf out

# Determine crate/binary name
crate=$(grep '^name[ \t]*=[ \t]*\".*\"' Cargo.toml | cut -d'=' -f 2 | tr -d '\t "')

for platform in $@; do
    case ${platform} in
        host)
            strip="strip"
            ext=".$(uname -m)"
            ;;
        rpi)
            # Check that the script is being used on x86_64
            [[ $(uname -m) != x86_64 ]] && echo Cross compiling is only supported on x86_64 && exit 1
            features="--no-default-features --features rpi"
            envs="-e PKG_CONFIG_PATH=/usr/arm-linux-gnueabihf/lib/pkgconfig:/opt/vc/lib/pkgconfig -e PKG_CONFIG_ALLOW_CROSS=1"
            target="arm-unknown-linux-gnueabihf"
            strip="arm-linux-gnueabihf-strip"
            ext=".rpi"
            ;;
        *)
            echo ${platform} is not supported
            echo Supported plaforms are \"$supported\"
            exit 1
            ;;
    esac

    # Prepare to build
    docker build ${crate}-builder

    # Compile, link and strip
    docker run $envs --rm --user "$(id -u)":"$(id -g)" \
        -v "$PWD":/build -w /build ${crate}-builder:latest sh -c "\
        cargo build ${target:+--target $target} $features --release && \
        $strip --strip-all target/$target/release/$crate"

    # Build a final release directory
    mkdir -p out
    cp target/$target/release/$crate "out/${crate}${ext}"
done

# Remember to throw in music, sync data, shaders, etc
cp -r resources out
