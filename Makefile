.PHONY: run-dev
run-dev:
	./target/release/ourspace --dev --ws-external

.PHONY: build-release
build-release:
	cargo build --release

.PHONY: purge-dev
purge-dev:
	./target/release/ourspace purge-chain --dev
	
.PHONY: init
init:
	./scripts/init.sh

.PHONY: test
test:
	SKIP_WASM_BUILD=1 cargo test --release --all