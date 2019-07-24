.PHONY: run

run: target/debug/libdemo.so
	cd target/debug; mehustin

target/debug/libdemo.so:
	cargo build

