.PHONY: run-dev
run-dev:
	./target/release/zerodao-node --dev --ws-external

.PHONY: build-release
build-release:
	cargo build --release

.PHONY: purge-dev
purge-dev:
	./target/release/zerodao-node purge-chain --dev
	
.PHONY: init
init:
	./scripts/init.sh

.PHONY: test
test:
	SKIP_WASM_BUILD=1 cargo test --release --all