build:
	cargo build
release:
	cargo build --release
test:
	cargo test
run:
	cargo run
docker:
	docker build -t dropshipping -f dockerfile .
clean:
	echo "Not implemented."
.PHONY: build test docker
