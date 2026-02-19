# Assemble all .vmra files in the examples directory
assemble-all:
    #!/usr/bin/env bash
    mkdir -p bin
    for file in examples/*.vmra; do \
        filename=$(basename "$file" .vmra); \
        echo "Processing $filename..."; \
        cargo run --quiet --release --bin assembler -- "$file" -o "bin/$filename.vmr"; \
    done