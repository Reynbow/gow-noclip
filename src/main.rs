mod tools;
use figlet_rs::FIGfont;

fn main() {
    let standard_font = FIGfont::standard().unwrap();
    let figure = standard_font.convert("GOWR No-Clip v1.1.3");
    if let Some(figure) = figure {
        println!("{}", figure);
    }

    // hook the program
    tools::handler::boot();
}
