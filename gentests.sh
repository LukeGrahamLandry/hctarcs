echo "!! Make sure to git submodule update --init --recursive"
cd vendor/scratch-compiler || exit
cargo build --release
cd ../.. || exit

build_vendor() {
  echo "=== Build Vendor $1 ==="
  cd "vendor/$1" || exit
  ../scratch-compiler/target/release/scratch-compiler src/main.scratch || exit
  cd ../.. || exit
  mv "vendor/$1/project.sb3" "target/$1.sb3" || exit

  yes | unzip "target/$1.sb3" -d "target/$1" || exit
  python -m json.tool "target/$1/project.json" > temp
  cat temp > "target/$1/project.json"
  rm temp
  # rm "target/$1.sb3"
}

build_test() {
  echo "=== Build Test $1 ==="
    cd "tests" || exit
    ../vendor/scratch-compiler/target/release/scratch-compiler "$1.scratch" || exit
    cd .. || exit
    mv "tests/project.sb3" "target/$1.sb3" || exit

    yes | unzip "target/$1.sb3" -d "target/$1" || exit
    python -m json.tool "target/$1/project.json" > temp
    cat temp > "target/$1/project.json"
    rm temp
}

build_test "sanity"
build_test "mandelbrot"

build_vendor "linrays"

cd "vendor/tres" || exit
./rebuild-fs  # TODO: should really just name the bin the right thing and use their make files
cd ../..
build_vendor "tres"
