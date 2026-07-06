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

fn position_global_addr(base: usize) -> usize {
    base + mappings().position_global_rva
}

fn forward_global_addr(base: usize) -> usize {
    base + mappings().forward_global_rva
}

fn directional_global_addr(base: usize) -> usize {
    base + mappings().directional_global_rva
}

pub fn change_height(process: &Process, base: usize, height_change: f32) -> Option<f32> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        position_global_addr(base),
        &[memory_mappings::POS_Y_OFFSET],
    ) {
        if let Some(val) = libmem::read_memory_ex::<f32>(process, addr) {
            let new_val = val + height_change;
            if libmem::write_memory_ex(process, addr, &new_val).is_some() {
                return Some(new_val);
            } else {
                println!("Failed to write new height at 0x{addr:X}");
            }
        } else {
            println!("Failed to read current height at 0x{addr:X}");
        }
    } else {
        println!("Failed to resolve height pointer chain.");
    }
    None
}

pub fn set_height(process: &Process, base: usize, value: f32) -> Option<()> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        position_global_addr(base),
        &[memory_mappings::POS_Y_OFFSET],
    ) {
        if libmem::write_memory_ex(process, addr, &value).is_some() {
            return Some(());
        } else {
            println!("Failed to write height at 0x{addr:X}");
        }
    } else {
        println!("Failed to resolve height pointer chain.");
    }
    None
}

pub fn get_height(process: &Process, base: usize) -> Option<f32> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        position_global_addr(base),
        &[memory_mappings::POS_Y_OFFSET],
    ) {
        if let Some(val) = libmem::read_memory_ex::<f32>(process, addr) {
            return Some(val);
        } else {
            println!("Failed to read current height at 0x{addr:X}");
        }
    } else {
        println!("Failed to resolve height pointer chain.");
    }
    None
}

#[allow(dead_code)]
pub fn get_forward(process: &Process, base: usize) -> Option<f32> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        forward_global_addr(base),
        memory_mappings::FORWARD_CHAIN,
    ) {
        if let Some(val) = libmem::read_memory_ex::<f32>(process, addr) {
            return Some(val);
        } else {
            println!("Failed to read current Forward / Backward at 0x{addr:X}");
        }
    } else {
        println!("Failed to resolve Forward / Backward pointer chain.");
    }
    None
}

#[allow(dead_code)]
fn get_left_right(process: &Process, base: usize) -> Option<f32> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        position_global_addr(base),
        &[memory_mappings::POS_X_OFFSET],
    ) {
        if let Some(val) = libmem::read_memory_ex::<f32>(process, addr) {
            return Some(val);
        } else {
            println!("Failed to read current left / right at 0x{addr:X}");
        }
    } else {
        println!("Failed to resolve left / right pointer chain.");
    }
    None
}

pub fn set_acceleration(process: &Process, base: usize, value: f32) -> Option<()> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        position_global_addr(base),
        &[memory_mappings::ACCEL_OFFSET],
    ) {
        if libmem::write_memory_ex(process, addr, &value).is_some() {
            return Some(());
        } else {
            println!("Failed to write acceleration at 0x{addr:X}");
        }
    } else {
        println!("Failed to resolve acceleration pointer chain.");
    }
    None
}

pub fn get_acceleration(process: &Process, base: usize) -> Option<f32> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        position_global_addr(base),
        &[memory_mappings::ACCEL_OFFSET],
    ) {
        if let Some(val) = libmem::read_memory_ex::<f32>(process, addr) {
            return Some(val);
        } else {
            println!("Failed to read current acceleration at 0x{addr:X}");
        }
    } else {
        println!("Failed to resolve acceleration pointer chain.");
    }
    None
}

pub fn move_left_right(process: &Process, base: usize, lr_change: f32) {
    if let Some(addr) = resolve_pointer_chain(
        process,
        position_global_addr(base),
        &[memory_mappings::POS_X_OFFSET],
    ) {
        read_write_memory(process, addr, lr_change);
    } else {
        println!("Failed to resolve left-right pointer");
    }
}

pub fn move_forward(process: &Process, base: usize, forward_change: f32) {
    if let Some(addr) = resolve_pointer_chain(
        process,
        forward_global_addr(base),
        memory_mappings::FORWARD_CHAIN,
    ) {
        read_write_memory(process, addr, forward_change);
    } else {
        println!("Failed to resolve forward pointer");
    }
}

fn read_write_memory(process: &Process, addr: usize, change: f32) -> Option<()> {
    if let Some(val) = libmem::read_memory_ex::<f32>(process, addr) {
        let new_val = val + change;
        match libmem::write_memory_ex(process, addr, &new_val) {
            Some(()) => Some(()),
            None => {
                println!("Failed to write value at 0x{addr:X}:");
                None
            }
        }
    } else {
        println!("Failed to read value at 0x{addr:X}");
        None
    }
}

pub fn turn_left_right(process: &Process, base: usize, lr_change: f32) {
    if let Some(addr) = resolve_pointer_chain(
        process,
        directional_global_addr(base),
        memory_mappings::DIRECTIONAL_CHAIN,
    ) {
        read_write_memory(process, addr, lr_change);
    } else {
        println!("Failed to resolve left-right rotational pointer");
    }
}

pub fn apply_directional_movement(
    process: &Process,
    base: usize,
    move_forward: f32,
    move_right: f32,
) -> Option<()> {
    let dir_addr = resolve_pointer_chain(
        process,
        directional_global_addr(base),
        memory_mappings::DIRECTIONAL_CHAIN,
    )?;

    let fx = read_memory_ex::<f32>(process, dir_addr)?;
    let fz = read_memory_ex::<f32>(process, dir_addr + 8)?;

    let angle = fx.atan2(fz);

    let forward = (angle.sin(), angle.cos());
    let right = (angle.cos(), -angle.sin());

    let delta_x = forward.0 * move_forward + right.0 * move_right;
    let delta_z = forward.1 * move_forward + right.1 * move_right;

    let x_addr = resolve_pointer_chain(
        process,
        position_global_addr(base),
        &[memory_mappings::POS_X_OFFSET],
    )?;
    let z_addr = resolve_pointer_chain(
        process,
        forward_global_addr(base),
        memory_mappings::FORWARD_CHAIN,
    )?;

    read_write_memory(process, x_addr, delta_x);
    read_write_memory(process, z_addr, delta_z);

    Some(())
}

pub fn init_mappings(process: &Process, base: usize) -> Result<(), String> {
    memory_resolver::resolve_mappings(process, base).map(|_| ())
}
