PROJECT := "valence-coprocessor-app"
COPROCESSOR := "coprocessor"
VERSION := "0.1.0"
PWD := $(CURDIR)

all: circuit domain program ## Build all targets.

clean: ## Clean all build artifacts and lock files
	cargo clean
	find docker/build -name Cargo.lock -type f -delete

circuit: docker-deploy ## Build the circuit ELF file.
	docker run --rm -it -v "$(PWD):/usr/src/app" $(PROJECT):$(VERSION)

domain: docker-deploy ## Build the domain WASM file.
	docker run --rm -it -v "$(PWD):/usr/src/app" $(PROJECT):$(VERSION) \
		cargo build --target wasm32-unknown-unknown --release --manifest-path ./docker/build/domain-wasm/Cargo.toml

program: docker-deploy ## Build the program WASM file.
	docker run --rm -it -v "$(PWD):/usr/src/app" $(PROJECT):$(VERSION) \
		cargo build --target wasm32-unknown-unknown --release --manifest-path ./docker/build/program-wasm/Cargo.toml

docker-deploy: ## Build the image.
	docker build -t $(PROJECT):$(VERSION) ./docker/deploy

coprocessor: ## Build and execute the coprocessor
	docker build -t $(COPROCESSOR):$(VERSION) ./docker/coprocessor
	docker run --rm -it --init -p 37281:37281 $(COPROCESSOR):$(VERSION)

help: ## Display this help screen
	@echo -e "\033[1;37m$(PROJECT) ($(VERSION))\033[0m"
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

.PHONY: help all circuit domain program docker-deploy coprocessor
