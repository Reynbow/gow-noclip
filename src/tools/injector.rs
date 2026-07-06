use crate::tools::memory_mappings;
use crate::tools::memory_resolver::{self, mappings};
use libmem::*;

pub fn resolve_pointer_chain(
    process: &Process,
    mut addr: usize,
    offsets: &[usize],
) -> Option<usize> {
    for offset in offsets {
        let ptr = libmem::read_memory_ex::<usize>(process, addr)?;
        if ptr < 0x10_000 {
            return None;
        }
        addr = ptr + offset;
    }
    Some(addr)
}

fn transform_base(process: &Process, base: usize) -> Option<usize> {
    resolve_pointer_chain(
        process,
        base + mappings().transform_global_rva,
        memory_mappings::TRANSFORM_CHAIN,
    )
}

fn transform_field(process: &Process, base: usize, field: usize) -> Option<usize> {
    Some(transform_base(process, base)? + field)
}

fn z_field(process: &Process, base: usize) -> Option<usize> {
    resolve_pointer_chain(
        process,
        base + mappings().transform_global_rva,
        memory_mappings::POS_Z_CHAIN,
    )
}

fn directional_field(process: &Process, base: usize) -> Option<usize> {
    resolve_pointer_chain(
        process,
        base + mappings().directional_global_rva,
        memory_mappings::DIRECTIONAL_CHAIN,
    )
}

pub fn change_height(process: &Process, base: usize, height_change: f32) -> Option<f32> {
    let addr = transform_field(process, base, memory_mappings::POS_Y_OFFSET)?;
    let val = libmem::read_memory_ex::<f32>(process, addr)?;
    let new_val = val + height_change;
    libmem::write_memory_ex(process, addr, &new_val).map(|_| new_val)
}

pub fn set_height(process: &Process, base: usize, value: f32) -> Option<()> {
    let addr = transform_field(process, base, memory_mappings::POS_Y_OFFSET)?;
    libmem::write_memory_ex(process, addr, &value).map(|_| ())
}

pub fn get_height(process: &Process, base: usize) -> Option<f32> {
    let addr = transform_field(process, base, memory_mappings::POS_Y_OFFSET)?;
    libmem::read_memory_ex::<f32>(process, addr)
}

#[allow(dead_code)]
pub fn get_forward(process: &Process, base: usize) -> Option<f32> {
    let addr = z_field(process, base)?;
    libmem::read_memory_ex::<f32>(process, addr)
}

pub fn set_acceleration(process: &Process, base: usize, value: f32) -> Option<()> {
    let addr = transform_field(process, base, memory_mappings::ACCEL_OFFSET)?;
    libmem::write_memory_ex(process, addr, &value).map(|_| ())
}

pub fn get_acceleration(process: &Process, base: usize) -> Option<f32> {
    let addr = transform_field(process, base, memory_mappings::ACCEL_OFFSET)?;
    libmem::read_memory_ex::<f32>(process, addr)
}

pub fn move_left_right(process: &Process, base: usize, lr_change: f32) {
    if let Some(addr) = transform_field(process, base, memory_mappings::POS_X_OFFSET) {
        read_write_memory(process, addr, lr_change);
    } else {
        println!("Failed to resolve X position field.");
    }
}

pub fn move_forward(process: &Process, base: usize, forward_change: f32) {
    if let Some(addr) = z_field(process, base) {
        read_write_memory(process, addr, forward_change);
    } else {
        println!("Failed to resolve Z position field.");
    }
}

fn read_write_memory(process: &Process, addr: usize, change: f32) -> Option<()> {
    let val = libmem::read_memory_ex::<f32>(process, addr)?;
    let new_val = val + change;
    libmem::write_memory_ex(process, addr, &new_val).map(|_| ())
}

pub fn turn_left_right(process: &Process, base: usize, lr_change: f32) {
    if let Some(addr) = directional_field(process, base) {
        read_write_memory(process, addr, lr_change);
    } else {
        println!("Failed to resolve directional field.");
    }
}

pub fn apply_directional_movement(
    process: &Process,
    base: usize,
    move_forward: f32,
    move_right: f32,
) -> Option<()> {
    let dir_addr = directional_field(process, base)?;

    let fx = read_memory_ex::<f32>(process, dir_addr)?;
    let fz = read_memory_ex::<f32>(process, dir_addr + 8)?;

    let angle = fx.atan2(fz);

    let forward = (angle.sin(), angle.cos());
    let right = (angle.cos(), -angle.sin());

    let delta_x = forward.0 * move_forward + right.0 * move_right;
    let delta_z = forward.1 * move_forward + right.1 * move_right;

    let x_addr = transform_field(process, base, memory_mappings::POS_X_OFFSET)?;
    let z_addr = z_field(process, base)?;

    read_write_memory(process, x_addr, delta_x);
    read_write_memory(process, z_addr, delta_z);

    Some(())
}

pub fn init_mappings(process: &Process, base: usize) -> Result<(), String> {
    memory_resolver::resolve_mappings(process, base).map(|_| ())
}
