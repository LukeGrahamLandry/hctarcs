


WASM_PATH := target/wasm32-unknown-unknown/release
WEB_DIST_PATH := ../LukeGrahamLandry.github.io/hctarcs

web_dist:
	cargo build --release --target wasm32-unknown-unknown --bin compiler
	mkdir -p $(WEB_DIST_PATH)/$(WASM_PATH)
	cp $(WASM_PATH)/compiler.wasm $(WEB_DIST_PATH)/$(WASM_PATH)/compiler.wasm
	cp index.html $(WEB_DIST_PATH)/index.html

.PHONY: release_web
