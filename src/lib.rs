pub(crate) mod compiler;
pub(crate) mod vm;

pub use compiler::{CompiledShader, compile, compile_fns, parse};
pub use vm::{
    CharSet,
    Color,
    ExternValue,
    RenderParams,
    TextMode,
    disassemble,
    disassemble_bytes,
    render,
    render_bytes,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const LICENSE: &str = include_str!("../LICENSE");
pub const CTSL_MAGIC: &[u8; 4] = b"CTSL";

/// Utility functions (to use outside of shaders, for example passing time() as the time parameter to render())
pub mod util {
    static START: std::sync::LazyLock<std::time::Instant> = std::sync::LazyLock::new(std::time::Instant::now);

    /// Strip ANSI escape codes from a string, returning the plain text content
    pub fn strip_ansi(s: &str) -> String {
        let mut result = String::new();
        let mut in_escape = false;
        for c in s.chars() {
            if in_escape {
                if c == 'm' {
                    in_escape = false;
                }
            } else if c == '\x1b' {
                in_escape = true;
            } else {
                result.push(c);
            }
        }
        result
    }

    /// Get current time in seconds since the program started
    #[inline]
    pub fn time() -> f32 {
        START.elapsed().as_secs_f32()
    }

    /// Convert 24-bit RGB to the nearest 8-bit ANSI color code (0-255)
    #[inline]
    pub fn ansi8_color(r: u8, g: u8, b: u8) -> u8 {
        let ri = (r as u16 * 5 / 255) as u8;
        let gi = (g as u16 * 5 / 255) as u8;
        let bi = (b as u16 * 5 / 255) as u8;
        16 + 36 * ri + 6 * gi + bi
    }

    // Check if the input file has changed since the output file was last generated
    pub fn has_file_changed(input: &std::path::Path, output: &std::path::Path) -> bool {
        let input_meta = std::fs::metadata(input);
        let output_meta = std::fs::metadata(output);
        if input_meta.is_err() || output_meta.is_err() {
            return true;
        }
        let input_mtime = input_meta.unwrap().modified();
        let output_mtime = output_meta.unwrap().modified();
        if input_mtime.is_err() || output_mtime.is_err() {
            return true;
        }
        input_mtime.unwrap() > output_mtime.unwrap()
    }

    /// Check if stdout is a tty (terminal)
    #[cfg(unix)]
    pub fn stdout_is_tty() -> bool {
        unsafe extern "C" {
            fn isatty(fd: i32) -> i32;
        }

        unsafe { isatty(1) == 1 }
    }

    /// Check if stdout is a tty (terminal)
    #[cfg(windows)]
    pub fn stdout_is_tty() -> bool {
        use std::ffi::c_void;

        const STD_OUTPUT_HANDLE: i32 = -11;

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetStdHandle(nStdHandle: i32) -> *mut c_void;
            fn GetConsoleMode(hConsoleHandle: *mut c_void, lpMode: *mut u32) -> i32;
        }

        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            if handle.is_null() || handle as isize == -1 {
                return false;
            }

            let mut mode = 0;
            GetConsoleMode(handle, &mut mode) != 0
        }
    }

    /// Prints ansi if in tty, otherwise returns the string without ansi
    #[inline]
    pub fn check_ansi(s: &str) -> String {
        if stdout_is_tty() {
            s.to_string()
        } else {
            strip_ansi(s)
        }
    }
}
pub use util as utils;
