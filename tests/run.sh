#!/bin/bash

set -e

echo "Cleaning up build artifacts..."
rm -rfv libbin/ obj/ bin/ logs/
mkdir -p libbin obj bin logs

echo "Cleanup complete."

echo "--- Compiling Libraries ---"
for file in libsrc/*.vmra; do
    [ -e "$file" ] || continue
    filename=$(basename "$file" .vmra)
    echo "Assembling $filename..."
    $VMR_ASSEMBLER "$file" -o "./libbin/${filename}.vmro" || { echo "Assembler failed on $file"; exit 1; }
done

echo -e "\n--- Compiling Test Sources ---"
for file in src/*.vmra; do
    [ -e "$file" ] || continue
    filename=$(basename "$file" .vmra)
    echo "Assembling $filename..."
    $VMR_ASSEMBLER "$file" -o "./obj/${filename}.vmra" || { echo "Assembler failed on $file"; exit 1; }
done

echo -e "\n--- Linking ---"
LIBS=""
for lib in libbin/*.vmro; do
    [ -e "$lib" ] || continue
    LIBS="$LIBS -i $lib"
done

for obj in obj/*.vmra; do
    [ -e "$obj" ] || continue
    filename=$(basename "$obj" .vmra)
    echo "Linking $filename..."
    $VMR_LINKER $LIBS -i "$obj" -o "bin/${filename}" || { echo "Linker failed on $obj"; exit 1; }
done

set +e

echo -e "\n--- Running Tests ---"
FAILED_TESTS=0

for test_bin in bin/*; do
    [ -e "$test_bin" ] || continue
    
    test_name=$(basename "$test_bin")
    echo -n "Running $test_name... "
    
    RUST_LOG=debug $VMR_RUNNER "./$test_bin" 2> "logs/${test_name}.txt"
    EXIT_CODE=$?
    
    if [ $EXIT_CODE -ne 0 ]; then
        echo -e "\e[31mFAILED\e[0m (Exit Code: $EXIT_CODE)"
        ((FAILED_TESTS++))
    else
        echo -e "\e[32mPASSED\e[0m"
    fi
done

echo -e "\n--- Summary ---"
if [ $FAILED_TESTS -eq 0 ]; then
    echo "All tests passed successfully!"
else
    echo "$FAILED_TESTS test(s) failed."
    exit 1
fi
