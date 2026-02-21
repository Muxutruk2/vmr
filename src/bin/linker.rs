use std::collections::HashMap;

pub struct Linker {
    objects: Vec<ObjectFile>,
    global_symbols: HashMap<String, u16>,
}

impl Default for Linker {
    fn default() -> Self {
        Self::new()
    }
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
        // Calculate current base address (end of the previous file)
        let base_address = self
            .objects
            .iter()
            .map(|obj| obj.bytecode.len() as u16)
            .sum();

        let object = ObjectFile::from_binary(data, name, base_address);

        for (label, offset) in object.exports.iter() {
            if self.global_symbols.contains_key(label) {
                println!("WARNING Label {label} is redefined by {name}");
            }

            self.global_symbols
                .insert(label.clone(), base_address + offset);
        }

        self.objects.push(object);
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
            for reloc_offset in &obj.internal_relocations {
                let idx = *reloc_offset as usize;

                let existing_addend =
                    u16::from_be_bytes([obj.bytecode[idx], obj.bytecode[idx + 1]]);

                let final_addr = existing_addend + obj.base_address;

                debug!("Patched local relocation: {existing_addend:04x} -> {final_addr:04x}");

                let patched_bytes = final_addr.to_be_bytes();
                obj.bytecode[idx] = patched_bytes[0];
                obj.bytecode[idx + 1] = patched_bytes[1];
            }

            for (label, reloc_offset) in &obj.external_relocations {
                let idx = *reloc_offset as usize;

                let final_addr = *self.global_symbols.get(label).ok_or(format!(
                    "Linker Error: Undefined symbol '{}' in {}",
                    label, obj.name
                ))?;

                let patched_bytes = final_addr.to_be_bytes();

                debug!("Patched external relocation: {final_addr:04x}");
                obj.bytecode[idx] = patched_bytes[0];
                obj.bytecode[idx + 1] = patched_bytes[1];
            }
        }

        for obj in self.objects {
            final_bytecode.extend(obj.bytecode);
        }

        let mut output = b"vmrx".to_vec();
        output.extend_from_slice(&final_bytecode);

        Ok(output)
    }
}

use clap::Parser;
use libvmr::ObjectFile;
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
