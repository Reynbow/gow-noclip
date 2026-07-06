use crate::tools::memory_mappings;
use crate::tools::sigscanner::{MemoryRead, ProcessImage};
use libmem::Process;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct GameMappings {
    pub transform_global_rva: usize,
    pub directional_global_rva: usize,
}

static RESOLVED_MAPPINGS: OnceLock<GameMappings> = OnceLock::new();

pub fn mappings() -> &'static GameMappings {
    RESOLVED_MAPPINGS
        .get()
        .expect("memory mappings were not resolved; call resolve_mappings() first")
}

pub fn resolve_mappings(process: &Process, base: usize) -> Result<&'static GameMappings, String> {
    if let Some(existing) = RESOLVED_MAPPINGS.get() {
        return Ok(existing);
    }

    let reader = ProcessReader { process };
    let image = ProcessImage::from_remote(&reader, base as u64)
        .ok_or_else(|| "failed to read GoWR.exe PE image for signature scanning".to_string())?;

    println!("Validating transform memory layout...");

    let mappings = discover_mappings(process, base, &image, &reader)?;
    if let (Some(x), Some(y), Some(z)) = (
        read_transform_field(process, base, &mappings, memory_mappings::POS_X_OFFSET),
        read_transform_field(process, base, &mappings, memory_mappings::POS_Y_OFFSET),
        read_z_field(process, base, &mappings),
    ) {
        println!(
            "Transform sample: x={x:.3} y={y:.3} z={z:.3}\n  transform global: 0x{:X}\n  directional global: 0x{:X}",
            mappings.transform_global_rva, mappings.directional_global_rva
        );
    }

    RESOLVED_MAPPINGS
        .set(mappings)
        .map_err(|_| "mappings already resolved".to_string())?;

    Ok(RESOLVED_MAPPINGS.get().unwrap())
}

pub fn scan_report(process: &Process, base: usize) -> Result<(), String> {
    let reader = ProcessReader { process };
    let image = ProcessImage::from_remote(&reader, base as u64)
        .ok_or_else(|| "failed to read GoWR.exe PE image".to_string())?;

    println!("=== GoWR Memory Scan Report ===");
    println!("Module base: 0x{base:X}");

    println!("\n--- Current layout (transform @ +0x2A0) ---");
    let current = GameMappings {
        transform_global_rva: memory_mappings::TRANSFORM_GLOBAL,
        directional_global_rva: memory_mappings::DIRECTIONAL_GLOBAL,
    };
    print_mapping_status(process, base, &current);

    println!("\n--- Legacy layout (broken position global) ---");
    print_legacy_status(process, base);

    if let Ok(found) = discover_mappings(process, base, &image, &reader) {
        println!("\n--- Resolved layout ---");
        print_mapping_status(process, base, &found);
    }

    Ok(())
}

fn print_mapping_status(process: &Process, base: usize, mappings: &GameMappings) {
    let transform_ok = validate_transform(process, base, mappings).is_ok();
    println!(
        "  transform 0x{:X} -> {}",
        mappings.transform_global_rva,
        if transform_ok { "OK" } else { "INVALID" }
    );

    if let Ok((x, y, z)) = validate_transform(process, base, mappings) {
        println!("    sample: x={x:.3} y={y:.3} z={z:.3}");
    }

    let dir_ok = validate_directional(process, base, mappings).is_ok();
    println!(
        "  directional 0x{:X} -> {}",
        mappings.directional_global_rva,
        if dir_ok { "OK" } else { "INVALID" }
    );
}

fn print_legacy_status(process: &Process, base: usize) {
    let ptr = read_memory::<usize>(process, base + memory_mappings::LEGACY_POSITION_GLOBAL);
    match ptr {
        Some(0) | None => println!(
            "  legacy position 0x{:X} -> null/broken",
            memory_mappings::LEGACY_POSITION_GLOBAL
        ),
        Some(p) => println!(
            "  legacy position 0x{:X} -> *global = 0x{p:X} (wrong object type)",
            memory_mappings::LEGACY_POSITION_GLOBAL
        ),
    }
}

fn discover_mappings(
    process: &Process,
    base: usize,
    _image: &ProcessImage,
    _reader: &ProcessReader<'_>,
) -> Result<GameMappings, String> {
    let mappings = GameMappings {
        transform_global_rva: memory_mappings::TRANSFORM_GLOBAL,
        directional_global_rva: memory_mappings::DIRECTIONAL_GLOBAL,
    };

    validate_transform(process, base, &mappings)?;
    validate_directional(process, base, &mappings)?;

    Ok(mappings)
}

fn validate_transform(process: &Process, base: usize, mappings: &GameMappings) -> Result<(f32, f32, f32), String> {
    let x = read_transform_field(process, base, mappings, memory_mappings::POS_X_OFFSET)
        .ok_or_else(|| "failed to read X".to_string())?;
    let y = read_transform_field(process, base, mappings, memory_mappings::POS_Y_OFFSET)
        .ok_or_else(|| "failed to read Y".to_string())?;
    let z = read_z_field(process, base, mappings)
        .ok_or_else(|| "failed to read Z".to_string())?;

    if !is_plausible_coord(x) || !is_plausible_coord(y) || !is_plausible_coord(z) {
        return Err(format!("implausible transform coords: x={x} y={y} z={z}"));
    }

    Ok((x, y, z))
}

fn validate_directional(process: &Process, base: usize, mappings: &GameMappings) -> Result<(), String> {
    let addr = resolve_chain(
        process,
        base + mappings.directional_global_rva,
        memory_mappings::DIRECTIONAL_CHAIN,
    )
    .ok_or_else(|| "directional pointer chain failed".to_string())?;

    let fx = read_memory::<f32>(process, addr).ok_or_else(|| "failed to read fx".to_string())?;
    let fz = read_memory::<f32>(process, addr + 8).ok_or_else(|| "failed to read fz".to_string())?;

    if !fx.is_finite() || !fz.is_finite() {
        return Err(format!("invalid direction vector: fx={fx} fz={fz}"));
    }

    Ok(())
}

fn read_transform_field(
    process: &Process,
    base: usize,
    mappings: &GameMappings,
    field: usize,
) -> Option<f32> {
    let transform = resolve_chain(
        process,
        base + mappings.transform_global_rva,
        memory_mappings::TRANSFORM_CHAIN,
    )?;
    read_memory::<f32>(process, transform + field)
}

fn read_z_field(process: &Process, base: usize, mappings: &GameMappings) -> Option<f32> {
    let addr = resolve_chain(
        process,
        base + mappings.transform_global_rva,
        memory_mappings::POS_Z_CHAIN,
    )?;
    read_memory::<f32>(process, addr)
}

fn resolve_chain(process: &Process, mut addr: usize, offsets: &[usize]) -> Option<usize> {
    for offset in offsets {
        let ptr = read_memory::<usize>(process, addr)?;
        if ptr < 0x10_000 {
            return None;
        }
        addr = ptr + offset;
    }
    Some(addr)
}

fn read_memory<T: Copy>(process: &Process, addr: usize) -> Option<T> {
    libmem::read_memory_ex::<T>(process, addr)
}

fn is_plausible_coord(v: f32) -> bool {
    v.is_finite() && v.abs() < 1_000_000.0
}

pub struct ProcessReader<'a> {
    process: &'a Process,
}

impl MemoryRead for ProcessReader<'_> {
    fn read_bytes(&self, addr: u64, buf: &mut [u8]) -> bool {
        let mut offset = 0usize;
        while offset < buf.len() {
            let read_addr = addr as usize + offset;
            let chunk_size = 8.min(buf.len() - offset);
            let Some(word) = libmem::read_memory_ex::<u64>(self.process, read_addr) else {
                return false;
            };
            let bytes = word.to_le_bytes();
            buf[offset..offset + chunk_size].copy_from_slice(&bytes[..chunk_size]);
            offset += chunk_size;
        }
        true
    }
}
