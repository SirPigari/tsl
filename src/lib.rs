pub(crate) mod compiler;
pub(crate) mod vm;

pub use compiler::{CompiledShader, compile, compile_fns, parse};
pub use vm::{CharSet, Color, RenderParams, TextMode, render, render_bytes};

/// Utility functions (to use outside of shaders, for example passing time() as the time parameter to render())
pub mod util {
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

    /// Get current time as seconds since UNIX epoch, as a float
    #[inline]
    pub fn time() -> f32 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f32()
    }

    /// Convert 24-bit RGB to the nearest 8-bit ANSI color code (0-255)
    #[inline]
    pub fn ansi8_color(r: u8, g: u8, b: u8) -> u8 {
        let ri = (r as u16 * 5 / 255) as u8;
        let gi = (g as u16 * 5 / 255) as u8;
        let bi = (b as u16 * 5 / 255) as u8;
        16 + 36 * ri + 6 * gi + bi
    }
}
pub use util as utils;
