use crate::tools::memory_mappings;
use libmem::*;

pub fn resolve_pointer_chain(
    process: &Process,
    mut addr: usize,
    offsets: &[usize],
) -> Option<usize> {
    for (_, offset) in offsets.iter().enumerate() {
        let ptr = libmem::read_memory_ex::<usize>(process, addr)?;
        // leaving this here for debugging purposes
        // println!(
        //     "Step {}: Read 0x{:X} -> 0x{:X} + 0x{:X}",
        //     i, addr, ptr, offset
        // );
        addr = ptr + offset;
    }
    Some(addr)
}

pub fn change_height(process: &Process, base: usize, height_change: f32) -> Option<f32> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        base + memory_mappings::HEIGHT_PTR.0,
        memory_mappings::HEIGHT_PTR.1,
    ) {
        if let Some(val) = libmem::read_memory_ex::<f32>(process, addr) {
            let new_val = val + height_change;
            if libmem::write_memory_ex(process, addr, &new_val).is_some() {
                return Some(new_val);
            } else {
                println!("Failed to write new height at 0x{:X}", addr);
            }
        } else {
            println!("Failed to read current height at 0x{:X}", addr);
        }
    } else {
        println!("Failed to resolve height pointer chain.");
    }
    None
}

pub fn set_height(process: &Process, base: usize, value: f32) -> Option<()> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        base + memory_mappings::HEIGHT_PTR.0,
        memory_mappings::HEIGHT_PTR.1,
    ) {
        if libmem::write_memory_ex(process, addr, &value).is_some() {
            return Some(());
        } else {
            println!("Failed to write height at 0x{:X}", addr);
        }
    } else {
        println!("Failed to resolve height pointer chain.");
    }
    None
}

pub fn get_height(process: &Process, base: usize) -> Option<f32> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        base + memory_mappings::HEIGHT_PTR.0,
        memory_mappings::HEIGHT_PTR.1,
    ) {
        if let Some(val) = libmem::read_memory_ex::<f32>(process, addr) {
            return Some(val);
        } else {
            println!("Failed to read current height at 0x{:X}", addr);
        }
    } else {
        println!("Failed to resolve height pointer chain.");
    }
    None
}

#[allow(dead_code)]
// currently not in use
pub fn get_forward(process: &Process, base: usize) -> Option<f32> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        base + memory_mappings::FORWARD_PTR.0,
        memory_mappings::FORWARD_PTR.1,
    ) {
        if let Some(val) = libmem::read_memory_ex::<f32>(process, addr) {
            return Some(val);
        } else {
            println!("Failed to read current Forward / Backward at 0x{:X}", addr);
        }
    } else {
        println!("Failed to resolve Forward / Backward pointer chain.");
    }
    None
}

#[allow(dead_code)]
// currently not in use
fn get_left_right(process: &Process, base: usize) -> Option<f32> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        base + memory_mappings::LEFT_RIGHT_PTR.0,
        memory_mappings::LEFT_RIGHT_PTR.1,
    ) {
        if let Some(val) = libmem::read_memory_ex::<f32>(process, addr) {
            return Some(val);
        } else {
            println!("Failed to read current left / right at 0x{:X}", addr);
        }
    } else {
        println!("Failed to resolve left / righjt pointer chain.");
    }
    None
}

pub fn set_acceleration(process: &Process, base: usize, value: f32) -> Option<()> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        base + memory_mappings::ACCELERATION_PTR.0,
        memory_mappings::ACCELERATION_PTR.1,
    ) {
        if libmem::write_memory_ex(process, addr, &value).is_some() {
            return Some(());
        } else {
            println!("Failed to write acceleration at 0x{:X}", addr);
        }
    } else {
        println!("Failed to resolve acceleration pointer chain.");
    }
    None
}

pub fn get_acceleration(process: &Process, base: usize) -> Option<f32> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        base + memory_mappings::ACCELERATION_PTR.0,
        memory_mappings::ACCELERATION_PTR.1,
    ) {
        if let Some(val) = libmem::read_memory_ex::<f32>(process, addr) {
            return Some(val);
        } else {
            println!("Failed to read current acceleration at 0x{:X}", addr);
        }
    } else {
        println!("Failed to resolve acceleration pointer chain.");
    }
    None
}

pub fn move_left_right(process: &Process, base: usize, lr_change: f32) {
    if let Some(addr) = resolve_pointer_chain(
        process,
        base + memory_mappings::LEFT_RIGHT_PTR.0,
        memory_mappings::LEFT_RIGHT_PTR.1,
    ) {
        read_write_memory(process, addr, lr_change);
    } else {
        println!("Failed to resolve left-right pointer");
    }
}

pub fn move_forward(process: &Process, base: usize, forward_change: f32) {
    if let Some(addr) = resolve_pointer_chain(
        process,
        base + memory_mappings::FORWARD_PTR.0,
        memory_mappings::FORWARD_PTR.1,
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
                println!("Failed to write value at 0x{:X}:", addr);
                None
            }
        }
    } else {
        println!("Failed to read value at 0x{:X}", addr);
        None
    }
}

pub fn turn_left_right(process: &Process, base: usize, lr_change: f32) {
    if let Some(addr) = resolve_pointer_chain(
        process,
        base + memory_mappings::DIRECTIONAL_PTR.0,
        memory_mappings::DIRECTIONAL_PTR.1,
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
        base + memory_mappings::DIRECTIONAL_PTR.0,
        memory_mappings::DIRECTIONAL_PTR.1,
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
        base + memory_mappings::LEFT_RIGHT_PTR.0,
        memory_mappings::LEFT_RIGHT_PTR.1,
    )?;
    let z_addr = resolve_pointer_chain(
        process,
        base + memory_mappings::FORWARD_PTR.0,
        memory_mappings::FORWARD_PTR.1,
    )?;

    read_write_memory(process, x_addr, delta_x);
    read_write_memory(process, z_addr, delta_z);

    Some(())
}
