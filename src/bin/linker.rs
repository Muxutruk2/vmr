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

    pub fn load_object(&mut self, name: &str, data: &[u8]) {
        let header_overhead = 3;

        let current_code_size: u16 = self
            .objects
            .iter()
            .map(|obj| obj.bytecode.len() as u16)
            .sum();

        let base_address = header_overhead + current_code_size;

        let object = ObjectFile::from_binary(data, name, base_address);

        for (label, offset) in object.exports.iter() {
            if self.global_symbols.contains_key(label) {
                warn!("WARNING Label {label} is redefined by {name}");
            }

            self.global_symbols
                .insert(label.clone(), base_address + offset);
        }

        self.objects.push(object);
    }

    pub fn link(mut self, entry_label: &str) -> Result<Vec<u8>, String> {
        let mut final_bytecode = Vec::new();

        let entry_addr = self
            .global_symbols
            .get(entry_label)
            .cloned()
            .ok_or(format!(
                "Linker Error: Entry point '{}' not found",
                entry_label
            ))?;

        let mut header = vec![Operation::JMP as u8];
        header.extend_from_slice(&entry_addr.to_be_bytes());

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
        output.extend(header);
        output.extend_from_slice(&final_bytecode);

        Ok(output)
    }
}

use clap::Parser;
use libvmr::{ObjectFile, Operation};
use log::{debug, error, info, warn};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(about = "Links .vmo files into a .vmr executable")]
struct Args {
    #[arg(short, long, value_name = "FILES", required = true)]
    inputs: Vec<PathBuf>,

    #[arg(short, long, value_name = "FILE", default_value = "out.vmr")]
    output: PathBuf,

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
                info!("Loading {}...", name);
                linker.load_object(&name, &data);
            }
            Err(e) => {
                error!("Error reading {}: {}", name, e);
                std::process::exit(1);
            }
        }
    }

    if !linker.global_symbols.contains_key(&args.entry) {
        warn!(
            "No entry point '{}' found. VM might not know where to start.",
            args.entry
        );
    }

    match linker.link(&args.entry) {
        Ok(binary) => {
            if binary.len() > 0xFFFF {
                error!("Binary size exceeds 16-bit address space ($0xFFFF$)!");
                std::process::exit(1);
            }

            if let Err(e) = fs::write(&args.output, binary) {
                error!("Error writing output: {}", e);
            } else {
                info!("Successfully linked to {}", args.output.display());
            }
        }
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    }
}
