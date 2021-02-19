#!/usr/bin/env bash
crate=$(grep '^name[ \t]*=[ \t]*\".*\"' Cargo.toml | cut -d'=' -f 2 | tr -d '\t "')
targetdir="${1:-release}"
cargo clean
docker build -t ${crate}-builder .
docker run --rm --user "$(id -u)":"$(id -g)" -v "$PWD":/usr/src/$crate -w /usr/src/$crate ${crate}-builder:latest cargo build --release
mkdir "$targetdir"
cp target/release/$crate "$targetdir"
strip --strip-all "$targetdir/$crate"
cp -r resources "$targetdir"
