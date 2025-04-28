all: build run ## Default target

build: wasm elf ## Build the artifacts

wasm: ## Build the WASM module
	@mkdir -p assets
	@cargo build --target wasm32-unknown-unknown \
		--release \
		--package valence-coprocessor-app-lib
	cp target/wasm32-unknown-unknown/release/valence_coprocessor_app_lib.wasm assets/demo.wasm

elf: ## Build the ELF program
	@mkdir -p assets
	@cd ./zkvm/circuit && \
		cargo prove build \
		--workspace-directory ./zkvm \
		--packages app-circuit
	@cargo run \
		--manifest-path ./zkvm/script/Cargo.toml -- \
		./assets/demo.elf

run: ## Execute the script
	@cargo run --package valence-coprocessor-app-script

help: ## Display this help screen
	@echo -e "\033[1;37m$(PROJECT) ($(VERSION))\033[0m"
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

.PHONY: help all wasm elf run
