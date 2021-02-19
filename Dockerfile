# Because of glibc version incompatibilities,
# this file prepares an image for building a more compatible release build.
FROM rust:slim-buster
RUN apt-get update && apt-get install -y build-essential libpulse-dev && rm -rf /var/lib/apt/lists/*
