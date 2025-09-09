use crate::tools::injector::{self, turn_left_right};
use crate::tools::memory_mappings;
use device_query::{DeviceQuery, DeviceState, Keycode};
use libmem::*;
use std::io::{self, Write, stdin, stdout};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub fn boot() {
    let process = match find_process(memory_mappings::GOW_PROC_NAME) {
        Some(p) => p,
        None => {
            println!(
                "Process '{}' not found. Exiting. Ensure the game is running and try again.",
                memory_mappings::GOW_PROC_NAME
            );

            await_for_user_interaction();
            return;
        }
    };

    let module = match find_module_ex(&process, memory_mappings::GOW_PROC_NAME) {
        Some(m) => m,
        None => {
            println!("Module not found. Exiting.");
            return;
        }
    };

    let base = module.base;
    println!("GoWR.exe base: 0x{:X}", base);

    // yes, i used GPT to generate this text
    // sue me
    println!(
        "
        \n================== Controls ==================\n\
        [G]      → Toggle No-Clip Mode (enables/disables input injection)\n\
        [W]      → Move forward (while No-Clip is active)\n\
        [S]      → Move backward\n\
        [A]      → Strafe left\n\
        [D]      → Strafe right\n\
        [Q]      → Rotate Left\n\
        [E]      → Rotate Right\n\
        [LSHIFT] → Ascend (move up)\n\
        [LCTRL]  → Descend (move down)\n\
        [L]      → Lock gravity (freeze vertical acceleration)\n\
        [U]      → Unlock gravity (restore falling behavior)\n\
        [X]      → Exit the program\n\
    \n\
        ▶ Type commands in console for instant movement:\n\
        - w/s/a/d/q/e → Same as above, instant position updates\n\
        - l/u         → Lock/unlock gravity manually\n\
        - x           → Exit cleanly\n\
    =================================================\n"
    );

    let movement_running = Arc::new(AtomicBool::new(true));
    let movement_running_clone = movement_running.clone();

    let proc_clone = process.clone();
    let move_clone = movement_running.clone();

    let device_state = DeviceState::new();
    let input_enabled = Arc::new(AtomicBool::new(false));
    let input_enabled_clone = input_enabled.clone();

    let target_height = Arc::new(std::sync::Mutex::new(None::<f32>));
    let target_height_clone = target_height.clone();

    let gravity_locked = Arc::new(AtomicBool::new(true)); // default is off, you dont wanna fall through the map
    let gravity_locked_clone = gravity_locked.clone();
    let gravity_locked_ui = gravity_locked.clone();

    // should kill the process if the game isn't running
    thread::spawn(move || {
        loop {
            if find_process(memory_mappings::GOW_PROC_NAME).is_none() {
                println!(
                    "Process '{}' exited. Shutting down.",
                    memory_mappings::GOW_PROC_NAME
                );
                movement_running_clone.store(false, Ordering::Relaxed);
                std::process::exit(0);
            }

            thread::sleep(Duration::from_secs(1));
        }
    });

    // no-clip logic begins here
    thread::spawn(move || {
        let mut g_was_down: bool = false;

        while move_clone.load(Ordering::Relaxed) {
            let keys = device_state.get_keys();

            if keys.contains(&Keycode::G) {
                if !g_was_down {
                    let new_state = !input_enabled_clone.load(Ordering::Relaxed);
                    input_enabled_clone.store(new_state, Ordering::Relaxed);
                    println!("Input toggled:{}", new_state);
                    g_was_down = true;

                    {
                        let mut target = target_height_clone.lock().unwrap();
                        *target = None;
                        println!("Target height reset.");
                    }
                }
            } else {
                g_was_down = false;
            }

            if input_enabled_clone.load(Ordering::Relaxed) {
                let keys = device_state.get_keys();

                let mut forward = 0.0;
                let mut strafe = 0.0;
                let mut rotate = 0.0;

                if keys.contains(&Keycode::W) {
                    forward = -0.2;
                }
                if keys.contains(&Keycode::S) {
                    forward = 0.2;
                }
                if keys.contains(&Keycode::D) {
                    strafe = 0.2;
                }
                if keys.contains(&Keycode::A) {
                    strafe = -0.2;
                }

                if keys.contains(&Keycode::Q) {
                    rotate = 0.03;
                }
                if keys.contains(&Keycode::E) {
                    rotate = -0.03;
                }

                if forward != 0.0 || strafe != 0.0 {
                    injector::apply_directional_movement(&proc_clone, base, forward, strafe);
                }

                if rotate != 0.0 {
                    turn_left_right(&proc_clone, base, rotate);
                }

                if keys.contains(&Keycode::L) {
                    gravity_locked.store(true, Ordering::Relaxed);
                    println!("Gravity locked (disabled).");
                }

                if keys.contains(&Keycode::U) {
                    gravity_locked.store(false, Ordering::Relaxed);
                    println!("Gravity unlocked (enabled).");
                }

                if keys.contains(&Keycode::LShift) {
                    if let Some(current) = injector::get_height(&proc_clone, base) {
                        let new_target = current + 1.5;
                        injector::change_height(&proc_clone, base, 0.5);
                        let mut target = target_height_clone.lock().unwrap();
                        *target = Some(new_target);
                    }
                }
                if keys.contains(&Keycode::LControl) {
                    if let Some(current) = injector::get_height(&proc_clone, base) {
                        let new_target = current - 0.5;
                        injector::change_height(&proc_clone, base, -0.5);
                        let mut target = target_height_clone.lock().unwrap();
                        *target = Some(new_target);
                    }
                }

                // hack to maintain target height
                if let Some(target) = *target_height_clone.lock().unwrap() {
                    if let Some(current) = injector::get_height(&proc_clone, base) {
                        if current < target {
                            injector::set_height(&proc_clone, base, target);
                        }

                        // acceleriation / gravity hack here
                        if gravity_locked_clone.load(Ordering::Relaxed) {
                            if let Some(current_accel) =
                                injector::get_acceleration(&proc_clone, base)
                            {
                                if current_accel.abs() > 0.01 {
                                    injector::set_acceleration(&proc_clone, base, 0.0);
                                }
                            }
                        }
                    }
                }
            }

            thread::sleep(Duration::from_micros(10));
        }
        println!("Movement thread stopped.");
    });

    // command loop starts here
    loop {
        print!("> ");
        stdout().flush().unwrap();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
        let cmd = input.trim();

        match cmd {
            "w" => injector::move_forward(&process, base, 5.0),
            "s" => injector::move_forward(&process, base, 5.5),
            "a" => injector::move_left_right(&process, base, -10.0),
            "d" => injector::move_left_right(&process, base, 10.0),
            "q" => {
                if let Some(height) = injector::change_height(&process, base, 20.0) {
                    println!("Height changed to {}", height);
                } else {
                    println!("Failed to change height.");
                }
            }
            "e" => {
                if let Some(height) = injector::change_height(&process, base, -20.0) {
                    println!("Height changed to {}", height);
                } else {
                    println!("Failed to change height.");
                }
            }
            "l" => {
                gravity_locked_ui.store(true, Ordering::Relaxed);
                println!("Gravity locked (disabled).");
            }
            "u" => {
                gravity_locked_ui.store(false, Ordering::Relaxed);
                println!("Gravity unlocked (enabled).");
            }
            "x" => {
                println!("Exiting...");
                break;
            }
            _ => {
                println!("Unknown command.");
            }
        }
    }
}

fn await_for_user_interaction() {
    print!("Press ENTER to exit...");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}
