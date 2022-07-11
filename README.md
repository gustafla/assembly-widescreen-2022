## Building

For building x86_64 and aarch64 release binaries, a podman container can be generated:  
`podman build -t rustbuild .`

Then, binaries can be built using it:  
`podman run -v .:/build -w /build rustbuild cargo build --release --target x86_64-unknown-linux-gnu`  
`podman run -v .:/build -w /build rustbuild cargo build --release --target aarch64-unknown-linux-gnu`  
`podman run -v .:/build -w /build rustbuild cargo build --release --target x86_64-pc-windows-gnu`
