/*
    Builds the bootloader partition

        cargo xtask build [--static] [--release]

    Copyup (default): copyup (padded to its reservation), then an ImageHeader, then the RAM-linked bootloader image.
    Static: the bootloader links at the start of flash, so its loadable bytes are the whole partition.
*/

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use object::elf::PT_LOAD;
use object::read::elf::{ElfFile32, ProgramHeader};
use object::{Object, ObjectSection};
use vgbl::image::header::ImageHeader;

const USAGE: &str = "usage: cargo xtask build [--static] [--release]";

const TARGET: &str = "thumbv7em-none-eabihf";

/// Package names, which are also the binary names
const BOOTLOADER: &str = "vgbl-stm32f4-bootloader";
const COPYUP: &str = "vgbl-stm32f4-copyup";

const FLASH_BASE: u32 = 0x0800_0000;

/// Flash reserved for copyup. Must match copyup/copyup.x
const COPYUP_SIZE: usize = 1024;

/// The bootloader partition is sector 0
const PARTITION_SIZE: usize = 16 * 1024;

/// Fill for gaps, matching erased flash
const ERASED: u8 = 0xFF;

/// The loadable contents of a linked ELF, laid out by load address
struct Image {
    base: u32,
    bytes: Vec<u8>,
    vector_table: u32,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let (command, flags) = args.split_first().ok_or(USAGE)?;
    if command != "build" {
        return Err(USAGE.into());
    }

    let mut static_ = false;
    let mut release = false;
    for flag in flags {
        match flag.as_str() {
            "--static" => static_ = true,
            "--release" => release = true,
            _ => return Err(format!("unknown flag {flag}\n{USAGE}")),
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let target_dir = env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| root.join("target"));
    let out_dir = target_dir.join(TARGET).join(if release { "release" } else { "debug" });

    let (partition, name) = if static_ {
        cargo_build(root, BOOTLOADER, &["--features", "static"], release)?;
        let bootloader = load(&out_dir.join(BOOTLOADER))?;
        if bootloader.base != FLASH_BASE {
            return Err(format!("bootloader links at {:#010x}, expected {FLASH_BASE:#010x}", bootloader.base));
        }
        (bootloader.bytes, "partition-static.bin")
    } else {
        cargo_build(root, COPYUP, &[], release)?;
        cargo_build(root, BOOTLOADER, &[], release)?;
        let copyup = load(&out_dir.join(COPYUP))?;
        let bootloader = load(&out_dir.join(BOOTLOADER))?;
        (assemble_copyup(copyup, bootloader)?, "partition.bin")
    };

    if partition.len() > PARTITION_SIZE {
        return Err(format!("partition is {} bytes, limit is {PARTITION_SIZE}", partition.len()));
    }

    let path = out_dir.join(name);
    fs::write(&path, &partition).map_err(|e| format!("writing {}: {e}", path.display()))?;
    println!("{}: {} of {PARTITION_SIZE} bytes", path.display(), partition.len());
    println!("flash with: probe-rs download --chip STM32F446RETx --binary-format bin --base-address {FLASH_BASE:#010x} {}", path.display());
    Ok(())
}

fn cargo_build(root: &Path, package: &str, extra: &[&str], release: bool) -> Result<(), String> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut command = Command::new(cargo);
    command.current_dir(root).args(["build", "--package", package, "--target", TARGET]).args(extra);
    if release {
        command.arg("--release");
    }

    let status = command.status().map_err(|e| format!("running cargo: {e}"))?;
    if !status.success() {
        return Err(format!("building {package} failed"));
    }
    Ok(())
}

fn load(path: &Path) -> Result<Image, String> {
    let data = fs::read(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let elf = ElfFile32::<object::Endianness>::parse(&*data).map_err(|e| format!("parsing {}: {e}", path.display()))?;
    let endian = elf.endian();

    // Segments with file contents are what ends up in flash, at their physical (load) address
    let mut segments = Vec::new();
    for header in elf.elf_program_headers() {
        if header.p_type(endian) != PT_LOAD || header.p_filesz(endian) == 0 {
            continue;
        }
        let bytes = header.data(endian, &*data).map_err(|_| format!("{}: bad segment", path.display()))?;
        segments.push((header.p_paddr(endian), bytes));
    }

    let base = segments.iter().map(|(addr, _)| *addr).min().ok_or(format!("{}: nothing to load", path.display()))?;
    let end = segments.iter().map(|(addr, bytes)| addr + bytes.len() as u32).max().unwrap();
    let mut image = vec![ERASED; (end - base) as usize];
    for (addr, bytes) in segments {
        let offset = (addr - base) as usize;
        image[offset..offset + bytes.len()].copy_from_slice(bytes);
    }

    let vector_table = elf
        .section_by_name(".vector_table")
        .ok_or(format!("{}: no .vector_table section", path.display()))?
        .address() as u32;

    Ok(Image { base, bytes: image, vector_table })
}

fn assemble_copyup(copyup: Image, bootloader: Image) -> Result<Vec<u8>, String> {
    if copyup.base != FLASH_BASE {
        return Err(format!("copyup links at {:#010x}, expected {FLASH_BASE:#010x}", copyup.base));
    }
    if copyup.bytes.len() > COPYUP_SIZE {
        return Err(format!("copyup is {} bytes, reservation is {COPYUP_SIZE}", copyup.bytes.len()));
    }

    // Copyup copies whole words
    let mut image = bootloader.bytes;
    image.resize(image.len().next_multiple_of(4), ERASED);

    let header = ImageHeader {
        load_addr: bootloader.base,
        size: image.len() as u32,
        vector_table_offset: bootloader.vector_table - bootloader.base,
    };

    let mut partition = copyup.bytes;
    partition.resize(COPYUP_SIZE, ERASED);
    partition.extend_from_slice(&header.to_bytes());
    partition.extend_from_slice(&image);

    println!(
        "copyup image: load {:#010x}, {} bytes, vector table +{:#x}",
        header.load_addr, header.size, header.vector_table_offset
    );
    Ok(partition)
}
