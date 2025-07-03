SOURCE_FILES := $(shell test -e src/ && find src -type f)
VERSION := $(shell sed -n 's,^version = \"\(.*\)\",\1,p' Cargo.toml)

policy.wasm: $(SOURCE_FILES) Cargo.*
	cargo build --target=wasm32-wasip1 --release
	cp target/wasm32-wasip1/release/*.wasm policy.wasm

annotated-policy.wasm: policy.wasm metadata.yml
	kwctl annotate -m metadata.yml -u README.md -o annotated-policy.wasm policy.wasm

.PHONY: fmt
fmt:
	cargo fmt --all -- --check

.PHONY: lint
lint:
	cargo clippy -- -D warnings

.PHONY: e2e-tests
e2e-tests: annotated-policy.wasm
	true
	# bats e2e.bats
	# Temporarily disable e2e-tests for the release; to run they need a kwctl with
	# the changes to the sdk and policy-evaluator, which are unreleased as these
	# run this specific policy in the integration tests.
	# The e2e-tests should be reinstated after the policy release.

.PHONY: test
test: fmt lint
	cargo test

.PHONY: clean
clean:
	cargo clean
	rm -f policy.wasm annotated-policy.wasm artifacthub-pkg.yml
