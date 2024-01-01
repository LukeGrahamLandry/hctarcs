WASM_PATH := target/wasm32-unknown-unknown/release
WEB_DIST_PATH := ../LukeGrahamLandry.github.io/hctarcs

TESTS = sanity mandelbrot

web:
	cargo build --release --target wasm32-unknown-unknown --bin compiler

web_dist: web
	mkdir -p $(WEB_DIST_PATH)/$(WASM_PATH)
	cp $(WASM_PATH)/compiler.wasm $(WEB_DIST_PATH)/$(WASM_PATH)/compiler.wasm
	cp index.html $(WEB_DIST_PATH)/index.html

# TODO: learn how to do a proper makefile
#       would be nice to have it generate a big image of the first frame of everything

# Make sure NOT to do the thing where you only rebuild if the source changed because im testing the compiler not the json blob.
test:
	cargo build --bin compiler
	./target/debug/compiler -i target/mandelbrot.sb3 -o out/gen/mandelbrot --check
	./target/debug/compiler -i target/linrays.sb3 -o out/gen/linrays --check
	./target/debug/compiler -i target/tres.sb3 -o out/gen/tres --check
	./target/debug/compiler -i target/sanity.sb3 -o out/gen/sanity --run

mandelbrot: target/mandelbrot/project.json
	cargo run --bin compiler
	cd out/gen/mandelbrot && cargo run --release

.PHONY: release_web test web
