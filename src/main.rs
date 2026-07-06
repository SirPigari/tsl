use std::{collections::HashMap, fs, process};
use tsl::CTSL_MAGIC;

fn usage(exit_code: i32) -> ! {
    eprintln!("usage: tslc [options]");
    eprintln!("usage: tslc [compile-options] <input.tsl> [output.ctsl]");
    eprintln!("usage: tslc <bc|bytecode> [bytecode-options] <input.ctsl>");
    eprintln!("usage: tslc render [render-options] <input.ctsl> <input.txt> <output.txt>");
    eprintln!("options:");
    eprintln!("  -h, --help       Show this help message");
    eprintln!("  -v, --version    Show version information");
    eprintln!("  -l, --license    Show license information");
    eprintln!("compile-options:");
    eprintln!("  -a, --ast        Show the AST of the compiled shader");
    eprintln!("bytecode-options:");
    eprintln!("  -o <output.bc>   Write disassembly to file instead of stdout");
    eprintln!("render-options:");
    eprintln!("  --time <seconds> Specify the time parameter for rendering (float)");
    eprintln!("  --mode <mode>    Specify text mode (ansi8, ansi24, ascii)");
    eprintln!("  --charset <set>  Specify character set (ascii, extended, unicode)");
    eprintln!("  --seed <seed>    Specify random seed (unsigned int)");
    eprintln!("  --extern <n=v>   Set an extern value (repeatable, supports tuples)");
    process::exit(exit_code);
}

fn parse_mode(s: &str) -> Option<tsl::TextMode> {
    match s {
        "ascii" => Some(tsl::TextMode::Ascii),
        "ansi8" => Some(tsl::TextMode::Ansi8),
        "ansi24" => Some(tsl::TextMode::Ansi24),
        _ => None,
    }
}

fn parse_charset(s: &str) -> Option<tsl::CharSet> {
    match s {
        "ascii" => Some(tsl::CharSet::Ascii),
        "extended" | "unicode" => Some(tsl::CharSet::Unicode),
        _ => None,
    }
}

fn parse_hex_color(s: &str) -> Option<tsl::Color> {
    let hex = s.strip_prefix('#')?;
    let digits = match hex.len() {
        3 => {
            let mut out = String::with_capacity(6);
            for ch in hex.chars() {
                out.push(ch);
                out.push(ch);
            }
            out
        }
        6 => hex.to_string(),
        _ => return None,
    };

    let r = u8::from_str_radix(&digits[0..2], 16).ok()?;
    let g = u8::from_str_radix(&digits[2..4], 16).ok()?;
    let b = u8::from_str_radix(&digits[4..6], 16).ok()?;
    Some(tsl::Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
    })
}

fn parse_char_literal(s: &str) -> Option<char> {
    let inner = s.strip_prefix('"')?.strip_suffix('"').or_else(|| s.strip_prefix('\'')?.strip_suffix('\''))?;
    let mut chars = inner.chars();
    let ch = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    Some(ch)
}

fn parse_char_value(s: &str) -> Option<char> {
    if let Some(ch) = parse_char_literal(s) {
        return Some(ch);
    }

    let n = s.trim().parse::<u32>().ok()?;
    char::from_u32(n)
}

fn split_top_level_commas(value: &str) -> Option<Vec<&str>> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut in_quote: Option<char> = None;
    let mut escaped = false;

    for (idx, ch) in value.char_indices() {
        if let Some(q) = in_quote {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == q {
                in_quote = None;
            }
            continue;
        }

        match ch {
            '"' | '\'' => in_quote = Some(ch),
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(value[start..idx].trim());
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    if in_quote.is_some() || depth != 0 {
        return None;
    }

    parts.push(value[start..].trim());
    Some(parts)
}

fn parse_extern_value(value: &str) -> Option<tsl::ExternValue> {
    let value = value.trim();
    let value = if value.starts_with('(') && value.ends_with(')') && value.len() >= 2 {
        &value[1..value.len() - 1]
    } else {
        value
    };

    if let Some(parts) = split_top_level_commas(value)
        && parts.len() > 1
    {
        match parts.as_slice() {
            [fg, bg] => {
                if let (Some(ch), Some(fg)) = (parse_char_value(fg), parse_extern_value(bg)) {
                    if let tsl::ExternValue::Color(c) = fg {
                        return Some((ch, c).into());
                    }
                }
                let fg = parse_extern_value(fg)?;
                let bg = parse_extern_value(bg)?;
                return match (fg, bg) {
                    (tsl::ExternValue::Color(fg), tsl::ExternValue::Color(bg)) => Some((fg, bg).into()),
                    _ => None,
                };
            }
            [a, fg, bg] => {
                let ch = parse_char_value(a)?;
                let fg = parse_extern_value(fg)?;
                let bg = parse_extern_value(bg)?;
                return match (fg, bg) {
                    (tsl::ExternValue::Color(fg), tsl::ExternValue::Color(bg)) => Some((ch, fg, bg).into()),
                    _ => None,
                };
            }
            _ => return None,
        }
    }

    if value.eq_ignore_ascii_case("true") {
        return Some(true.into());
    }
    if value.eq_ignore_ascii_case("false") {
        return Some(false.into());
    }
    if let Some(color) = parse_hex_color(value) {
        return Some(color.into());
    }
    if let Some(ch) = parse_char_literal(value) {
        return Some(ch.into());
    }
    if let Ok(n) = value.parse::<f32>() {
        return Some(n.into());
    }
    None
}

fn fail(msg: &str) -> ! {
    eprintln!("error: {msg}");
    process::exit(1)
}

fn has_ctsl_magic(path: &str) -> bool {
    match fs::read(path) {
        Ok(data) => data.starts_with(CTSL_MAGIC),
        Err(_) => false,
    }
}

fn default_compile_output(input: &str) -> String {
    if let Some((stem, _)) = input.rsplit_once('.') {
        format!("{stem}.ctsl")
    } else {
        format!("{input}.ctsl")
    }
}

fn run_compile(args: &[String]) {
    let mut show_ast = false;
    let mut positional: Vec<&str> = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-a" | "--ast" => show_ast = true,
            s if s.starts_with('-') => fail(&format!("unknown compile option {s}")),
            s => positional.push(s),
        }
    }

    if positional.is_empty() || positional.len() > 2 {
        usage(1);
    }

    let input = positional[0];
    let output = if positional.len() == 2 {
        positional[1].to_string()
    } else {
        default_compile_output(input)
    };

    let src = fs::read_to_string(input).unwrap_or_else(|e| fail(&format!("read error: {e}")));
    if show_ast {
        let ast = tsl::parse(&src).unwrap_or_else(|e| fail(&format!("parse error: {e}")));
        println!("{:#?}", ast);
    }

    let shader = tsl::compile(&src).unwrap_or_else(|e| fail(&format!("compile error: {e}")));
    let bytes = shader.to_bytes();
    fs::write(&output, &bytes).unwrap_or_else(|e| fail(&format!("write error: {e}")));
    println!("{} -> {} ({} bytes)", input, output, bytes.len());
}

fn run_bytecode(args: &[String]) {
    let mut out_file: Option<&str> = None;
    let mut positional: Vec<&str> = Vec::new();
    let mut i = 0usize;

    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                if i >= args.len() {
                    fail("-o requires an output path");
                }
                out_file = Some(args[i].as_str());
            }
            s if s.starts_with('-') => fail(&format!("unknown bytecode option {s}")),
            s => positional.push(s),
        }
        i += 1;
    }

    if positional.len() != 1 {
        usage(1);
    }

    let input = positional[0];
    let data = fs::read(input).unwrap_or_else(|e| fail(&format!("read error: {e}")));
    let text = tsl::disassemble_bytes(&data).unwrap_or_else(|e| fail(&format!("bytecode error: {e}")));

    if let Some(path) = out_file {
        fs::write(path, text.as_bytes()).unwrap_or_else(|e| fail(&format!("write error: {e}")));
        println!("{} -> {}", input, path);
    } else {
        print!("{}", text);
    }
}

fn run_render(args: &[String]) {
    let mut params = tsl::RenderParams::default();
    let mut externs: HashMap<String, tsl::ExternValue> = HashMap::new();
    let mut positional: Vec<&str> = Vec::new();
    let mut i = 0usize;

    while i < args.len() {
        match args[i].as_str() {
            "--time" => {
                i += 1;
                if i >= args.len() {
                    fail("--time requires a value");
                }
                params.time = args[i]
                    .parse::<f32>()
                    .unwrap_or_else(|_| fail("--time must be a float"));
            }
            "--mode" => {
                i += 1;
                if i >= args.len() {
                    fail("--mode requires a value");
                }
                params.mode = parse_mode(&args[i]).unwrap_or_else(|| fail("--mode must be ascii, ansi8, or ansi24"));
            }
            "--charset" => {
                i += 1;
                if i >= args.len() {
                    fail("--charset requires a value");
                }
                params.charset = parse_charset(&args[i]).unwrap_or_else(|| fail("--charset must be ascii, extended, or unicode"));
            }
            "--seed" => {
                i += 1;
                if i >= args.len() {
                    fail("--seed requires a value");
                }
                params.seed = args[i]
                    .parse::<u32>()
                    .unwrap_or_else(|_| fail("--seed must be an unsigned integer"));
            }
            "--extern" => {
                i += 1;
                if i >= args.len() {
                    fail("--extern requires a name=value pair");
                }
                let spec = &args[i];
                let Some((name, value)) = spec.split_once('=') else {
                    fail("--extern must be in the form name=value");
                };
                let parsed = parse_extern_value(value)
                    .unwrap_or_else(|| fail(&format!("could not parse extern value {value}")));
                externs.insert(name.to_string(), parsed);
            }
            s if s.starts_with('-') => fail(&format!("unknown render option {s}")),
            s => positional.push(s),
        }
        i += 1;
    }

    if positional.len() != 3 {
        usage(1);
    }

    let shader_path = positional[0];
    let text_path = positional[1];
    let output_path = positional[2];

    let text_in = fs::read_to_string(text_path)
        .unwrap_or_else(|e| fail(&format!("input text read error: {e}")));

    params.externs = externs;

    let rendered = if has_ctsl_magic(shader_path) {
        let ctsl = fs::read(shader_path).unwrap_or_else(|e| fail(&format!("read error: {e}")));
        tsl::render_bytes(&ctsl, &text_in, &params)
            .unwrap_or_else(|e| fail(&format!("render error: {e}")))
    } else {
        let src = fs::read_to_string(shader_path)
            .unwrap_or_else(|e| fail(&format!("shader source read error: {e}")));
        let shader = tsl::compile(&src)
            .unwrap_or_else(|e| fail(&format!("compile error: {e}")));
        tsl::render(&shader, &text_in, &params)
            .unwrap_or_else(|e| fail(&format!("render error: {e}")))
    };
    fs::write(output_path, rendered).unwrap_or_else(|e| fail(&format!("write error: {e}")));
    println!("rendered {} with {} -> {}", shader_path, text_path, output_path);
}

fn run_auto(args: &[String]) {
    if args.is_empty() {
        usage(1);
    }

    let input = &args[0];
    if has_ctsl_magic(input) {
        if args.iter().any(|a| a == "-o") {
            run_bytecode(args);
            return;
        }
        if args.len() == 1 {
            run_bytecode(args);
            return;
        }
        if args.len() == 2 {
            let data = fs::read(input).unwrap_or_else(|e| fail(&format!("read error: {e}")));
            let text = tsl::disassemble_bytes(&data)
                .unwrap_or_else(|e| fail(&format!("bytecode error: {e}")));
            fs::write(&args[1], text.as_bytes()).unwrap_or_else(|e| fail(&format!("write error: {e}")));
            println!("{} -> {}", input, args[1]);
            return;
        }
        run_render(args);
        return;
    }

    run_compile(args);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage(1);
    }

    match args[0].as_str() {
        "-h" | "--help" => usage(0),
        "-v" | "--version" => {
            println!("TSL (Text Shader Language) v{} by Mia a.k.a Markofwitch", tsl::VERSION);
        }
        "-l" | "--license" => {
            print!("{}", tsl::LICENSE);
        }
        "bc" | "bytecode" => run_bytecode(&args[1..]),
        "render" => run_render(&args[1..]),
        _ => run_auto(&args),
    }
}
