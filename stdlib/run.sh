#!/usr/bin/env bash

set -e

# Ensure output directories exist
mkdir -p obj tests/obj tests/bin

echo "--- Assembling Source Files ---"
for file in src/*.vmra; do
    [ -e "$file" ] || continue
    filename=$(basename "$file" .vmra)
    output="obj/lib${filename}.vmro"
    
    echo "Assembling $filename -> $output"
    $VMR_ASSEMBLER "$file" -o "$output" || { echo "Failed to assemble $file"; exit 1; }
done

echo -e "\n--- Assembling Test Files ---"
for file in tests/*.vmra; do
    [ -e "$file" ] || continue
    filename=$(basename "$file" .vmra)
    output="tests/obj/${filename}.vmro"
    
    echo "Assembling $filename -> $output"
    $VMR_ASSEMBLER "$file" -o "$output" || { echo "Failed to assemble $file"; exit 1; }
done

echo -e "\n--- Linking Tests with Libraries ---"
for test_obj in tests/obj/*.vmro; do
    [ -e "$test_obj" ] || continue
    
    # 1. Get the filename without extension (e.g., numprint_dec)
    test_name=$(basename "$test_obj" .vmro)
    
    # 2. Extract prefix before the first underscore (e.g., numprint)
    lib_prefix="${test_name%%_*}"
    
    # 3. Define the expected library object path
    matching_lib="obj/lib${lib_prefix}.vmro"
    
    # 4. Link if the library exists
    if [ -f "$matching_lib" ]; then
        echo "Linking $test_name with library $lib_prefix..."
        $VMR_LINKER -i "$matching_lib" -i "$test_obj" -o "tests/bin/${test_name}" || { echo "Linking failed for $test_name"; exit 1; }
    else
        echo "Warning: No library found for $test_name (Expected $matching_lib). Skipping..."
    fi
done

echo -e "\nBuild complete. Binaries are located in tests/bin/"
