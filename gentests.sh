cd vendor/scratch-compiler || exit
cargo build
cd ../linrays || exit
echo "Building Ray Tracer..."
../scratch-compiler/target/debug/scratch-compiler src/main.scratch
mv project.sb3 ../../target/linrays.sb3
cd ../..
yes | unzip target/linrays.sb3 -d target/linrays
python -m json.tool target/linrays/project.json > temp
cat temp > target/linrays/project.json
rm temp
