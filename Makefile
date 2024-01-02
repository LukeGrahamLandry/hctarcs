WASM_PATH := target/wasm32-unknown-unknown/release
WEB_DIST_PATH := ../LukeGrahamLandry.github.io/hctarcs

# TODO: build wasm demos from my tests as well.
web:
	cargo build --release --target wasm32-unknown-unknown --package compiler --lib --no-default-features

web_dist: web
	mkdir -p $(WEB_DIST_PATH)/$(WASM_PATH)
	cp $(WASM_PATH)/compiler.wasm $(WEB_DIST_PATH)/$(WASM_PATH)/compiler.wasm
	cp index.html $(WEB_DIST_PATH)/index.html

# TODO: learn how to do a proper makefile.
# TODO: would be nice to have it generate a big image of the first frame of everything

# Make sure NOT to do the thing where you only rebuild if the source changed because im testing the compiler not the json blob.
test: target/sanity.sb3 target/mandelbrot.sb3 target/stamp_pos.sb3
	cargo build --bin compiler
	./target/debug/compiler -i target/mandelbrot.sb3 -o out/gen/mandelbrot --check --deny-poly
	./target/debug/compiler -i target/linrays.sb3 -o out/gen/linrays --check --deny-poly
	./target/debug/compiler -i target/tres.sb3 -o out/gen/tres --check
	./target/debug/compiler -i target/stamp_pos.sb3 -o out/gen/stamp_pos --check --deny-poly
	./target/debug/compiler -i target/sanity.sb3 -o out/gen/sanity --debug

mandelbrot: target/mandelbrot/project.json
	cargo run --bin compiler -- -i target/mandelbrot.sb3 -o out/gen/mandelbrot --deny-poly --run

tres:
	cargo run --bin compiler -- -i target/tres.sb3 -o out/gen/tres --run

linrays:
	cargo run --bin compiler -- -i target/linrays.sb3 -o out/gen/linrays --run

stamp_pos: target/stamp_pos.sb3
	cargo run --bin compiler -- -i target/stamp_pos.sb3 -o out/gen/stamp_pos --run

# Note: scratch-compiler needs you in the right current directory to find asset files (and scratch needs you to have a texture even if you're just using the pen).
target/sanity.sb3: tests/sanity.scratch
	cd tests && ../vendor/scratch-compiler/target/release/scratch-compiler sanity.scratch
	mv "./tests/project.sb3" "target/sanity.sb3"

target/mandelbrot.sb3: tests/mandelbrot.scratch
	cd tests && ../vendor/scratch-compiler/target/release/scratch-compiler mandelbrot.scratch
	mv "./tests/project.sb3" "target/mandelbrot.sb3"

target/stamp_pos.sb3: tests/stamp_pos.scratch
	cd tests && ../vendor/scratch-compiler/target/release/scratch-compiler stamp_pos.scratch
	mv "./tests/project.sb3" "target/stamp_pos.sb3"

.PHONY: release_web test web mandelbrot tres linrays
