.PHONY: all build release check examples clean

all: build

build:
	cargo build

release:
	cargo build --release

check:
	cargo check

examples: build
	./target/debug/best examples/factorial.best
	./target/debug/best examples/fibonacci.best
	./target/debug/best examples/while_loop.best
	./target/debug/best examples/strings.best

clean:
	cargo clean
