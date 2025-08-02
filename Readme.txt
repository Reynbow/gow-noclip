GoWR No-Clip Tool (God of War: Ragnarok - PC)
=============================================

A Rust-based no-clip movement tool for God of War: Ragnarok (PC).

This tool lets you move Kratos in real time by directly manipulating game memory.
It was built as a reverse engineering and Rust experiment — because why should C++ have all the fun?

----------------------------------------------------
✨ Features
----------------------------------------------------

✅ WASD – Move Kratos (camera-relative)
✅ LSHIFT / LCTRL – Move up / down vertically
✅ Q / E – Rotate camera left / right
✅ G – Toggle No-Clip Mode (enable/disable live movement)
✅ L – Lock gravity (freeze vertical movement)
✅ U – Unlock gravity (restore gravity)
✅ Target height lock – Prevents vertical drift during flight
✅ Command console – Control with typed commands
✅ Atomic threading – Ultra responsive controls
✅ Uses libmem (by rdbo) for memory editing

----------------------------------------------------
🖥️ Console Commands
----------------------------------------------------

While the tool is running, you can also type commands directly into the terminal:

Command   | Action
----------|-------------------------------
w         | Move forward
s         | Move backward
a         | Move left
d         | Move right
q         | Rotate left
e         | Rotate right
l         | Lock gravity
u         | Unlock gravity
x         | Exit the program

----------------------------------------------------
🚀 How to Use
----------------------------------------------------

1. **Launch God of War: Ragnarok (PC)**
   - Wait until Kratos is in-game and controllable.

2. **Run the No-Clip Tool**
   - Open a terminal or double-click the compiled executable.

3. **Use hotkeys to fly around**
   - Press G to toggle no-clip mode ON/OFF.
   - Use W/A/S/D to move, Q/E to rotate, LSHIFT/LCTRL to fly up/down.

4. **Type commands into the terminal**
   - If you prefer command input, just type `w`, `l`, or `x` into the console window.

5. **Exit anytime**
   - Press `x` in the terminal or close the window.

----------------------------------------------------
⚠️ Notes
----------------------------------------------------

- Built for **educational purposes** — don’t use it in multiplayer.
- Only works on the **PC version** of God of War: Ragnarok.
- May require **Administrator permissions** to access game memory.
- Works best in **borderless windowed** mode.

----------------------------------------------------
👨‍💻 Credits
----------------------------------------------------

- Made with ❤️ in Rust
- Memory manipulation powered by [libmem](https://github.com/rdbo/libmem)
- Created to learn reverse engineering and real-time game control in Rust
