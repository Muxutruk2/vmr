use std::collections::HashMap;

struct ObjectFile {
    name: String,
    exports: HashMap<String, u16>,
    relocations: Vec<(String, u16)>,
    bytecode: Vec<u8>,
    base_address: u16,
}

pub struct Linker {
    objects: Vec<ObjectFile>,
    global_symbols: HashMap<String, u16>,
}

impl Linker {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            global_symbols: HashMap::new(),
        }
    }

    /// Stage 1: Parse the .vmo bytes and track the global base address
    pub fn load_object(&mut self, name: &str, data: &[u8]) {
        let mut cursor = 4; // Skip "vmro"

        // Calculate current base address (end of the previous file)
        let base_address = self
            .objects
            .iter()
            .map(|obj| obj.bytecode.len() as u16)
            .sum();

        // Parse Exports
        let export_count = u16::from_be_bytes([data[cursor], data[cursor + 1]]);
        cursor += 2;
        let mut exports = HashMap::new();
        for _ in 0..export_count {
            let (label, bytes_read) = self.read_string(&data[cursor..]);
            cursor += bytes_read;
            let offset = u16::from_be_bytes([data[cursor], data[cursor + 1]]);
            cursor += 2;
            // Map local offset to Global Address
            self.global_symbols
                .insert(label.clone(), base_address + offset);
            exports.insert(label, offset);
        }

        // Parse Relocations
        let reloc_count = u16::from_be_bytes([data[cursor], data[cursor + 1]]);
        cursor += 2;
        let mut relocations = Vec::new();
        for _ in 0..reloc_count {
            let (label, bytes_read) = self.read_string(&data[cursor..]);
            cursor += bytes_read;
            let offset = u16::from_be_bytes([data[cursor], data[cursor + 1]]);
            cursor += 2;
            relocations.push((label, offset));
        }

        let bytecode = data[cursor..].to_vec();

        self.objects.push(ObjectFile {
            name: name.to_string(),
            exports,
            relocations,
            bytecode,
            base_address,
        });
    }

    /// Stage 2 & 3: Patch placeholders and merge into one .vmr
    pub fn link(mut self) -> Result<Vec<u8>, String> {
        let mut final_bytecode = Vec::new();

        let total_size: usize = self.objects.iter().map(|o| o.bytecode.len()).sum();
        if total_size > 0xFFFF {
            return Err(format!(
                "Linker Error: Total size {} exceeds 64KB limit",
                total_size
            ));
        }

        for obj in &mut self.objects {
            for (label, offset) in &obj.relocations {
                debug!(
                    "Symbol {label} at offset {offset:x} requested by {}",
                    obj.name
                );

                // Find the symbol in the global map.
                // This map contains (Object_Base_Address + Local_Label_Offset)
                let global_addr = self.global_symbols.get(label).ok_or(format!(
                    "Linker Error: Undefined symbol '{}' requested by {}",
                    label, obj.name
                ))?;

                debug!("Global_addr: {global_addr:x}");

                let addr_bytes = global_addr.to_be_bytes();
                let idx = *offset as usize;

                // SAFETY: Ensure the relocation offset is actually within this object's bytecode
                if idx + 1 >= obj.bytecode.len() {
                    return Err(format!(
                        "Linker Error: Relocation offset {} out of bounds in {}",
                        idx, obj.name
                    ));
                }

                debug!(
                    "Current address: {:x}{:x}",
                    obj.bytecode[idx],
                    obj.bytecode[idx + 1]
                );

                // UNIFIED PATCH:
                // If local: overwrites local offset (e.g., 0x0030) with global (e.g., 0x1030)
                // If foreign: overwrites 0x0000 with global (e.g., 0x0500)
                obj.bytecode[idx] = addr_bytes[0];
                obj.bytecode[idx + 1] = addr_bytes[1];

                debug!(
                    "Patched address: {:x}{:x}",
                    obj.bytecode[idx],
                    obj.bytecode[idx + 1]
                );
            }

            final_bytecode.extend_from_slice(&obj.bytecode);
        }

        // 3. Final Executable Construction
        let mut output = b"vmrx".to_vec(); // 'x' for executable
        output.extend_from_slice(&final_bytecode);

        Ok(output)
    }

    fn read_string(&self, data: &[u8]) -> (String, usize) {
        let len = u16::from_be_bytes([data[0], data[1]]) as usize;
        let s = String::from_utf8_lossy(&data[2..2 + len]).to_string();
        (s, len + 2)
    }
}

use clap::Parser;
use log::debug;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(about = "Links .vmo files into a .vmr executable")]
struct Args {
    /// Input object files (.vmo)
    #[arg(short, long, value_name = "FILES", required = true)]
    inputs: Vec<PathBuf>,

    /// Output executable file (.vmr)
    #[arg(short, long, value_name = "FILE", default_value = "out.vmr")]
    output: PathBuf,

    /// Optional: Set a specific entry point label (default is START)
    #[arg(short, long, default_value = "START")]
    entry: String,
}

fn main() {
    let args = Args::parse();
    let mut linker = Linker::new();

    env_logger::builder().format_timestamp(None).init();

    for path in args.inputs {
        let name = path.to_string_lossy().to_string();
        match fs::read(&path) {
            Ok(data) => {
                println!("Loading {}...", name);
                linker.load_object(&name, &data);
            }
            Err(e) => {
                eprintln!("Error reading {}: {}", name, e);
                std::process::exit(1);
            }
        }
    }

    // 2. Check for the entry point
    if !linker.global_symbols.contains_key(&args.entry) {
        eprintln!(
            "Warning: No entry point '{}' found. VM might not know where to start.",
            args.entry
        );
    }

    // 3. Link and check for 16-bit overflow
    match linker.link() {
        Ok(binary) => {
            if binary.len() > 0xFFFF {
                eprintln!("Linker Error: Binary size exceeds 16-bit address space ($0xFFFF$)!");
                std::process::exit(1);
            }

            if let Err(e) = fs::write(&args.output, binary) {
                eprintln!("Error writing output: {}", e);
            } else {
                println!("Successfully linked to {}", args.output.display());
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
