use std::{fs, process};

fn usage() -> ! {
    eprintln!("usage: tslc <input.tsl> [output.ctsl]");
    process::exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
    }
    let input = &args[1];
    let output = if args.len() >= 3 {
        args[2].clone()
    } else {
        input.strip_suffix(".tsl").unwrap_or(input).to_string() + ".ctsl"
    };
    let src = fs::read_to_string(input).unwrap_or_else(|e| {
        eprintln!("read error: {e}");
        process::exit(1)
    });
    let shader = tsl::compile(&src).unwrap_or_else(|e| {
        eprintln!("compile error: {e}");
        process::exit(1)
    });
    let bytes = shader.to_bytes();
    fs::write(&output, &bytes).unwrap_or_else(|e| {
        eprintln!("write error: {e}");
        process::exit(1)
    });
    println!("{} -> {} ({} bytes)", input, output, bytes.len());
}
