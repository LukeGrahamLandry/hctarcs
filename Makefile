


WASM_PATH := target/wasm32-unknown-unknown/release
WEB_DIST_PATH := ../LukeGrahamLandry.github.io/hctarcs

web_dist:
	cargo build --release --target wasm32-unknown-unknown --bin compiler
	mkdir -p $(WEB_DIST_PATH)/$(WASM_PATH)
	cp $(WASM_PATH)/compiler.wasm $(WEB_DIST_PATH)/$(WASM_PATH)/compiler.wasm
	cp index.html $(WEB_DIST_PATH)/index.html

# TODO: learn how to do a proper makefile
#       would be nice to have it generate a big image of the first frame of everything
target/sanity/project.json: tests/sanity.scratch
	./gentests.sh

target/mandelbrot/project.json: tests/mandelbrot.scratch
	./gentests.sh

test: target/sanity/project.json
	cargo run --bin compiler
	cd target/scratch_out_sanity && cargo run

mandelbrot: target/mandelbrot/project.json
	cargo run --bin compiler
	cd target/scratch_out_mandelbrot && cargo run

.PHONY: release_web test
