


WASM_PATH := target/wasm32-unknown-unknown/release
WEB_DIST_PATH := ../LukeGrahamLandry.github.io/hctarcs

web_dist:
	cargo build --release --target wasm32-unknown-unknown --bin compiler
	mkdir -p $(WEB_DIST_PATH)/$(WASM_PATH)
	cp $(WASM_PATH)/compiler.wasm $(WEB_DIST_PATH)/$(WASM_PATH)/compiler.wasm
	cp index.html $(WEB_DIST_PATH)/index.html

target/sanity/project.json: tests/sanity.scratch
	./gentests.sh

test: target/sanity/project.json
	cargo run --bin compiler
	cd target/scratch_out_sanity && cargo run

.PHONY: release_web test
