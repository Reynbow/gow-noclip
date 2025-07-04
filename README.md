# GoWR No-Clip Tool

A Rust-based no-clip movement tool for *God of War: Ragnarok* (PC).  
Allows you to control player position in memory to enable free movement.
It's a project I built to attempt to teach myself how to reverse engineer and manipulate memory in rust.
Everyone is doing it in C++, we gotta bring rust some love, man.

## Features

- W/A/S/D — Move forward/back/left/right
- Q/E — Adjust vertical position (height)
- G — Toggle input control
- Command console with extra movement commands
- Atomic no-clip thread with target height locking

## Dependencies
It uses libmem - by rdbo

## Build

Make sure you have Rust installed:  
https://www.rust-lang.org/tools/install

Then build the project:

```bash
cargo build --release
