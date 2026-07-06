mod tools;

fn main() {
    println!(
        "+-----------------------------------+\n\
         | GOW Ragnarok No-Clip v1.2.1      |\n\
         | (c) 2025 alexanderdth            |\n\
         +-----------------------------------+"
    );

    // hook the program
    tools::handler::boot();
}
