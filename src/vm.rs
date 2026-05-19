use crate::compiler::{CompiledShader, Op, ReturnType};
use std::cell::Cell;

thread_local! {
    static RNG: Cell<u32> = const { Cell::new(0x9e3779b9u32) };
}

#[inline(always)]
fn fast_rand() -> f32 {
    RNG.with(|r| {
        let mut x = r.get();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        r.set(x);
        (x >> 8) as f32 * (1.0 / 16_777_216.0)
    })
}

/// RGB color, each channel 0.0..1.0
#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Color {
    #[inline]
    fn clamp(self) -> Self {
        Self {
            r: self.r.clamp(0.0, 1.0),
            g: self.g.clamp(0.0, 1.0),
            b: self.b.clamp(0.0, 1.0),
        }
    }
}

fn hue2rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> Color {
    if s == 0.0 {
        return Color { r: l, g: l, b: l };
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    Color {
        r: hue2rgb(p, q, h + 1.0 / 3.0),
        g: hue2rgb(p, q, h),
        b: hue2rgb(p, q, h - 1.0 / 3.0),
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color {
    if s == 0.0 {
        return Color { r: v, g: v, b: v };
    }
    let i = (h * 6.0) as i32;
    let f = h * 6.0 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    match i % 6 {
        0 => Color { r: v, g: t, b: p },
        1 => Color { r: q, g: v, b: p },
        2 => Color { r: p, g: v, b: t },
        3 => Color { r: p, g: q, b: v },
        4 => Color { r: t, g: p, b: v },
        _ => Color { r: v, g: p, b: q },
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Val {
    F(f32),
    C(Color),
    G(Color, char),
}

impl Val {
    fn f(self) -> f32 {
        match self {
            Val::F(v) => v,
            Val::C(c) => (c.r + c.g + c.b) / 3.0,
            Val::G(c, _) => (c.r + c.g + c.b) / 3.0,
        }
    }
    fn c(self) -> Color {
        match self {
            Val::C(c) => c,
            Val::F(v) => Color { r: v, g: v, b: v },
            Val::G(c, _) => c,
        }
    }
}

const MAX_LOCALS: usize = 64;
const MAX_STACK: usize = 256;
const MAX_DEPTH: usize = 32;

struct Frame {
    fn_idx: usize,
    ip: usize,
    locals: [Val; MAX_LOCALS],
}

impl Frame {
    fn new(fn_idx: usize) -> Self {
        Self {
            fn_idx,
            ip: 0,
            locals: [Val::F(0.0); MAX_LOCALS],
        }
    }
}

pub struct Vm<'a> {
    shader: &'a CompiledShader,
    stack: Vec<Val>,
    frames: Vec<Frame>,
}

impl<'a> Vm<'a> {
    pub fn new(shader: &'a CompiledShader) -> Self {
        Self {
            shader,
            stack: Vec::with_capacity(MAX_STACK),
            frames: Vec::with_capacity(MAX_DEPTH),
        }
    }

    #[inline(always)]
    fn push(&mut self, v: Val) {
        self.stack.push(v);
    }
    #[inline(always)]
    fn pop(&mut self) -> Val {
        self.stack.pop().unwrap_or(Val::F(0.0))
    }

    fn read_u8(code: &[u8], ip: &mut usize) -> u8 {
        let v = code[*ip];
        *ip += 1;
        v
    }
    fn read_i16(code: &[u8], ip: &mut usize) -> i16 {
        let v = i16::from_le_bytes([code[*ip], code[*ip + 1]]);
        *ip += 2;
        v
    }
    fn read_f32(code: &[u8], ip: &mut usize) -> f32 {
        let v = f32::from_le_bytes([code[*ip], code[*ip + 1], code[*ip + 2], code[*ip + 3]]);
        *ip += 4;
        v
    }

    pub(crate) fn run(&mut self, fn_idx: usize, args: &[Val]) -> Val {
        self.frames.clear();
        self.stack.clear();
        let mut frame = Frame::new(fn_idx);
        for (i, &a) in args.iter().enumerate() {
            frame.locals[i] = a;
        }
        loop {
            let f = &self.shader.fns[frame.fn_idx];
            let code = &f.code;
            let op = code[frame.ip];
            frame.ip += 1;
            match op {
                o if o == Op::PushF as u8 => {
                    let v = Self::read_f32(code, &mut frame.ip);
                    self.push(Val::F(v));
                }
                o if o == Op::PushC as u8 => {
                    let r = Self::read_f32(code, &mut frame.ip);
                    let g = Self::read_f32(code, &mut frame.ip);
                    let b = Self::read_f32(code, &mut frame.ip);
                    self.push(Val::C(Color { r, g, b }));
                }
                o if o == Op::Pop as u8 => {
                    self.pop();
                }
                o if o == Op::Load as u8 => {
                    let s = Self::read_u8(code, &mut frame.ip) as usize;
                    self.push(frame.locals[s]);
                }
                o if o == Op::Store as u8 => {
                    let s = Self::read_u8(code, &mut frame.ip) as usize;
                    frame.locals[s] = self.pop();
                }
                o if o == Op::AddF as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(a + b));
                }
                o if o == Op::SubF as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(a - b));
                }
                o if o == Op::MulF as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(a * b));
                }
                o if o == Op::DivF as u8 => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a, b) {
                        (Val::C(ca), Val::C(cb)) => {
                            let r = if cb.r == 0.0 { 0.0 } else { ca.r / cb.r };
                            let g = if cb.g == 0.0 { 0.0 } else { ca.g / cb.g };
                            let b = if cb.b == 0.0 { 0.0 } else { ca.b / cb.b };
                            self.push(Val::C(Color { r, g, b }));
                        }
                        _ => {
                            let bf = b.f();
                            let af = a.f();
                            self.push(Val::F(if bf == 0.0 { 0.0 } else { af / bf }));
                        }
                    }
                }
                o if o == Op::ModF as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(a % b));
                }
                o if o == Op::NegF as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(-a));
                }
                o if o == Op::AbsF as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.abs()));
                }
                o if o == Op::AddC as u8 => {
                    let b = self.pop().c();
                    let a = self.pop().c();
                    self.push(Val::C(Color {
                        r: a.r + b.r,
                        g: a.g + b.g,
                        b: a.b + b.b,
                    }));
                }
                o if o == Op::SubC as u8 => {
                    let b = self.pop().c();
                    let a = self.pop().c();
                    self.push(Val::C(Color {
                        r: a.r - b.r,
                        g: a.g - b.g,
                        b: a.b - b.b,
                    }));
                }
                o if o == Op::MulC as u8 => {
                    let b = self.pop().c();
                    let a = self.pop().c();
                    self.push(Val::C(Color {
                        r: a.r * b.r,
                        g: a.g * b.g,
                        b: a.b * b.b,
                    }));
                }
                o if o == Op::MulCF as u8 => {
                    let t = self.pop().f();
                    let a = self.pop().c();
                    self.push(Val::C(Color {
                        r: a.r * t,
                        g: a.g * t,
                        b: a.b * t,
                    }));
                }
                o if o == Op::DivC as u8 => {
                    let b = self.pop().c();
                    let a = self.pop().c();
                    let r = if b.r == 0.0 { 0.0 } else { a.r / b.r };
                    let g = if b.g == 0.0 { 0.0 } else { a.g / b.g };
                    let bv = if b.b == 0.0 { 0.0 } else { a.b / b.b };
                    self.push(Val::C(Color { r, g, b: bv }));
                }
                o if o == Op::EqF as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(if a == b { 1.0 } else { 0.0 }));
                }
                o if o == Op::NeF as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(if a != b { 1.0 } else { 0.0 }));
                }
                o if o == Op::LtF as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(if a < b { 1.0 } else { 0.0 }));
                }
                o if o == Op::GtF as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(if a > b { 1.0 } else { 0.0 }));
                }
                o if o == Op::LeF as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(if a <= b { 1.0 } else { 0.0 }));
                }
                o if o == Op::GeF as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(if a >= b { 1.0 } else { 0.0 }));
                }
                o if o == Op::AndF as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(if a != 0.0 && b != 0.0 { 1.0 } else { 0.0 }));
                }
                o if o == Op::OrF as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(if a != 0.0 || b != 0.0 { 1.0 } else { 0.0 }));
                }
                o if o == Op::NotF as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(if a == 0.0 { 1.0 } else { 0.0 }));
                }
                o if o == Op::Sin as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.sin()));
                }
                o if o == Op::Cos as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.cos()));
                }
                o if o == Op::Tan as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.tan()));
                }
                o if o == Op::Asin as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.asin()));
                }
                o if o == Op::Acos as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.acos()));
                }
                o if o == Op::Atan as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.atan()));
                }
                o if o == Op::Atan2 as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(a.atan2(b)));
                }
                o if o == Op::Sqrt as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.sqrt()));
                }
                o if o == Op::Pow as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(a.powf(b)));
                }
                o if o == Op::Exp as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.exp()));
                }
                o if o == Op::Log as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.ln()));
                }
                o if o == Op::Log2 as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.log2()));
                }
                o if o == Op::Floor as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.floor()));
                }
                o if o == Op::Ceil as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.ceil()));
                }
                o if o == Op::Round as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.round()));
                }
                o if o == Op::Fract as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.fract()));
                }
                o if o == Op::Min2 as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(a.min(b)));
                }
                o if o == Op::Max2 as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(a.max(b)));
                }
                o if o == Op::Clamp as u8 => {
                    let hi = self.pop().f();
                    let lo = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(a.clamp(lo, hi)));
                }
                o if o == Op::Mix as u8 => {
                    let t = self.pop();
                    let b = self.pop();
                    let a = self.pop();
                    match (a, b) {
                        (Val::C(ca), Val::C(cb)) => {
                            let tf = t.f();
                            self.push(Val::C(Color {
                                r: ca.r + (cb.r - ca.r) * tf,
                                g: ca.g + (cb.g - ca.g) * tf,
                                b: ca.b + (cb.b - ca.b) * tf,
                            }));
                        }
                        _ => {
                            let tf = t.f();
                            let af = a.f();
                            let bf = b.f();
                            self.push(Val::F(af + (bf - af) * tf));
                        }
                    }
                }
                o if o == Op::Step as u8 => {
                    let x = self.pop().f();
                    let e = self.pop().f();
                    self.push(Val::F(if x < e { 0.0 } else { 1.0 }));
                }
                o if o == Op::Smoothstep as u8 => {
                    let x = self.pop().f();
                    let e1 = self.pop().f();
                    let e0 = self.pop().f();
                    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
                    self.push(Val::F(t * t * (3.0 - 2.0 * t)));
                }
                o if o == Op::Sign as u8 => {
                    let a = self.pop().f();
                    self.push(Val::F(a.signum()));
                }
                o if o == Op::Length2 as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F((a * a + b * b).sqrt()));
                }
                o if o == Op::Rgb as u8 => {
                    let b = self.pop().f();
                    let g = self.pop().f();
                    let r = self.pop().f();
                    self.push(Val::C(Color {
                        r: r / 255.0,
                        g: g / 255.0,
                        b: b / 255.0,
                    }));
                }
                o if o == Op::Rgba as u8 => {
                    let _a = self.pop().f();
                    let b = self.pop().f();
                    let g = self.pop().f();
                    let r = self.pop().f();
                    self.push(Val::C(Color {
                        r: r / 255.0,
                        g: g / 255.0,
                        b: b / 255.0,
                    }));
                }
                o if o == Op::Hsl as u8 => {
                    let l = self.pop().f();
                    let s = self.pop().f();
                    let h = self.pop().f();
                    self.push(Val::C(hsl_to_rgb(h, s, l)));
                }
                o if o == Op::Hsv as u8 => {
                    let v = self.pop().f();
                    let s = self.pop().f();
                    let h = self.pop().f();
                    self.push(Val::C(hsv_to_rgb(h, s, v)));
                }
                o if o == Op::Gray as u8 => {
                    let v = self.pop().f();
                    self.push(Val::C(Color { r: v, g: v, b: v }));
                }
                o if o == Op::Mix2C as u8 => {
                    let t = self.pop().f();
                    let b = self.pop().c();
                    let a = self.pop().c();
                    self.push(Val::C(Color {
                        r: a.r + (b.r - a.r) * t,
                        g: a.g + (b.g - a.g) * t,
                        b: a.b + (b.b - a.b) * t,
                    }));
                }
                o if o == Op::GetR as u8 => {
                    let c = self.pop().c();
                    self.push(Val::F(c.r));
                }
                o if o == Op::GetG as u8 => {
                    let c = self.pop().c();
                    self.push(Val::F(c.g));
                }
                o if o == Op::GetB as u8 => {
                    let c = self.pop().c();
                    self.push(Val::F(c.b));
                }
                o if o == Op::IsSpace as u8 => {
                    let code = self.pop().f() as u32;
                    let ch = char::from_u32(code).unwrap_or('\0');
                    self.push(Val::F(if ch.is_whitespace() { 1.0 } else { 0.0 }));
                }
                o if o == Op::IsDigit as u8 => {
                    let code = self.pop().f() as u32;
                    let ch = char::from_u32(code).unwrap_or('\0');
                    self.push(Val::F(if ch.is_ascii_digit() { 1.0 } else { 0.0 }));
                }
                o if o == Op::IsAlpha as u8 => {
                    let code = self.pop().f() as u32;
                    let ch = char::from_u32(code).unwrap_or('\0');
                    self.push(Val::F(if ch.is_ascii_alphabetic() { 1.0 } else { 0.0 }));
                }
                o if o == Op::IsUpper as u8 => {
                    let code = self.pop().f() as u32;
                    let ch = char::from_u32(code).unwrap_or('\0');
                    self.push(Val::F(if ch.is_ascii_uppercase() { 1.0 } else { 0.0 }));
                }
                o if o == Op::IsLower as u8 => {
                    let code = self.pop().f() as u32;
                    let ch = char::from_u32(code).unwrap_or('\0');
                    self.push(Val::F(if ch.is_ascii_lowercase() { 1.0 } else { 0.0 }));
                }
                o if o == Op::Jmp as u8 => {
                    let off = Self::read_i16(code, &mut frame.ip);
                    frame.ip = (frame.ip as i64 + off as i64) as usize;
                }
                o if o == Op::JmpZ as u8 => {
                    let off = Self::read_i16(code, &mut frame.ip);
                    if self.pop().f() == 0.0 {
                        frame.ip = (frame.ip as i64 + off as i64) as usize;
                    }
                }
                o if o == Op::Ret as u8 => {
                    let ret = self.pop();
                    if let Some(parent) = self.frames.pop() {
                        frame = parent;
                        self.push(ret);
                    } else {
                        return ret;
                    }
                }
                o if o == Op::Call as u8 => {
                    let idx = Self::read_u8(code, &mut frame.ip) as usize;
                    let argc = Self::read_u8(code, &mut frame.ip) as usize;
                    let mut new_frame = Frame::new(idx);
                    let base = self.stack.len() - argc;
                    for i in 0..argc {
                        new_frame.locals[i] = self.stack[base + i];
                    }
                    self.stack.truncate(base);
                    let old = std::mem::replace(&mut frame, new_frame);
                    self.frames.push(old);
                }
                o if o == Op::Rand as u8 => {
                    self.push(Val::F(fast_rand()));
                }
                o if o == Op::RandBetween as u8 => {
                    let b = self.pop().f();
                    let a = self.pop().f();
                    self.push(Val::F(a + fast_rand() * (b - a)));
                }
                o if o == Op::MakeGlyph as u8 => {
                    let b = self.pop();
                    let a = self.pop();
                    let (col, ch) = match (a, b) {
                        (Val::C(col), _) => (col, char::from_u32(b.f() as u32).unwrap_or('\0')),
                        (_, Val::C(col)) => (col, char::from_u32(a.f() as u32).unwrap_or('\0')),
                        _ => (a.c(), char::from_u32(b.f() as u32).unwrap_or('\0')),
                    };
                    self.push(Val::G(col, ch));
                }
                _ => { /* skip */ }
            }
        }
    }
}

/// How to encode color in output text.
#[derive(Debug, Clone, Copy)]
pub enum TextMode {
    /// Plain ASCII, no color codes.
    Ascii,
    /// ANSI 8-bit color (256 colors).
    Ansi8,
    /// ANSI 24-bit true color.
    Ansi24,
}

/// Output character set.
#[derive(Debug, Clone, Copy)]
pub enum CharSet {
    /// 7-bit ASCII glyphs.
    Ascii,
    /// Full Unicode block glyphs.
    Unicode,
}

pub struct RenderParams {
    pub mode: TextMode,
    pub charset: CharSet,
    pub cols: u32,
    pub time: f32,
}

impl Default for RenderParams {
    fn default() -> Self {
        Self {
            mode: TextMode::Ansi24,
            charset: CharSet::Ascii,
            cols: 0,
            time: 0.0,
        }
    }
}

#[derive(Clone, Copy)]
struct Glyph {
    ch: char,
    original: Color,
}

#[inline]
fn color_from_ansi16(idx: u8) -> Color {
    let (r, g, b) = match idx {
        0 => (0, 0, 0),
        1 => (128, 0, 0),
        2 => (0, 128, 0),
        3 => (128, 128, 0),
        4 => (0, 0, 128),
        5 => (128, 0, 128),
        6 => (0, 128, 128),
        7 => (192, 192, 192),
        8 => (128, 128, 128),
        9 => (255, 0, 0),
        10 => (0, 255, 0),
        11 => (255, 255, 0),
        12 => (0, 0, 255),
        13 => (255, 0, 255),
        14 => (0, 255, 255),
        _ => (255, 255, 255),
    };
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
    }
}

#[inline]
fn color_from_xterm256(n: u8) -> Color {
    let (r, g, b) = match n {
        0..=15 => {
            let c = color_from_ansi16(n);
            return c;
        }
        16..=231 => {
            let i = n - 16;
            let r = (i / 36) % 6;
            let g = (i / 6) % 6;
            let b = i % 6;
            let c = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            (c(r), c(g), c(b))
        }
        _ => {
            let v = 8 + (n - 232) * 10;
            (v, v, v)
        }
    };
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
    }
}

fn apply_sgr(seq: &str, fg: &mut Color) {
    let src = if seq.is_empty() { "0" } else { seq };
    let mut it = src.split(';').map(|s| s.parse::<u16>().unwrap_or(0));
    while let Some(code) = it.next() {
        match code {
            0 | 39 => {
                *fg = Color {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                }
            }
            30..=37 => *fg = color_from_ansi16((code - 30) as u8),
            90..=97 => *fg = color_from_ansi16((code - 90 + 8) as u8),
            38 => match it.next() {
                Some(5) => {
                    if let Some(n) = it.next() {
                        *fg = color_from_xterm256(n as u8);
                    }
                }
                Some(2) => {
                    if let (Some(r), Some(g), Some(b)) = (it.next(), it.next(), it.next()) {
                        *fg = Color {
                            r: r.min(255) as f32 / 255.0,
                            g: g.min(255) as f32 / 255.0,
                            b: b.min(255) as f32 / 255.0,
                        };
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn parse_ansi_glyph_lines(text: &str) -> Vec<Vec<Glyph>> {
    if !text.as_bytes().contains(&0x1B) {
        let white = Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        };
        return text
            .split('\n')
            .map(|line| {
                line.chars()
                    .map(|ch| Glyph {
                        ch,
                        original: white,
                    })
                    .collect()
            })
            .collect();
    }

    let mut lines: Vec<Vec<Glyph>> = vec![Vec::new()];
    let mut fg = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
    };

    let mut i = 0usize;
    while i < text.len() {
        let bytes = text.as_bytes();
        if bytes[i] == 0x1B {
            if i + 1 < text.len() && bytes[i + 1] == b'[' {
                let mut j = i + 2;
                while j < text.len() && bytes[j] != b'm' {
                    j += 1;
                }
                if j < text.len() && bytes[j] == b'm' {
                    apply_sgr(&text[i + 2..j], &mut fg);
                    i = j + 1;
                    continue;
                }
            }
            i += 1;
            continue;
        }

        let mut iter = text[i..].chars();
        if let Some(ch) = iter.next() {
            i += ch.len_utf8();
            if ch == '\n' {
                lines.push(Vec::new());
            } else {
                if let Some(line) = lines.last_mut() {
                    line.push(Glyph { ch, original: fg });
                }
            }
        } else {
            break;
        }
    }

    lines
}

#[inline(always)]
fn push_dec(out: &mut String, n: u8) {
    if n >= 100 {
        out.push((b'0' + n / 100) as char);
    }
    if n >= 10 {
        out.push((b'0' + (n / 10) % 10) as char);
    }
    out.push((b'0' + n % 10) as char);
}

fn emit_char(out: &mut String, ch: char, r: u8, g: u8, b: u8, mode: TextMode) {
    match mode {
        TextMode::Ascii => {
            out.push(ch);
        }
        TextMode::Ansi8 => {
            out.push_str("\x1b[38;5;");
            push_dec(out, crate::util::ansi8_color(r, g, b));
            out.push('m');
            out.push(ch);
        }
        TextMode::Ansi24 => {
            out.push_str("\x1b[38;2;");
            push_dec(out, r);
            out.push(';');
            push_dec(out, g);
            out.push(';');
            push_dec(out, b);
            out.push('m');
            out.push(ch);
        }
    }
}

#[inline(always)]
fn apply_result(
    out: &mut String,
    result: Val,
    ch: char,
    orig: Color,
    mode: TextMode,
    ret: ReturnType,
) {
    match result {
        Val::G(col, new_ch) => {
            let col = col.clamp();
            emit_char(
                out,
                new_ch,
                (col.r * 255.0) as u8,
                (col.g * 255.0) as u8,
                (col.b * 255.0) as u8,
                mode,
            );
        }
        _ => match ret {
            ReturnType::Char => {
                let new_ch = char::from_u32(result.f() as u32).unwrap_or(ch);
                let orig = orig.clamp();
                emit_char(
                    out,
                    new_ch,
                    (orig.r * 255.0) as u8,
                    (orig.g * 255.0) as u8,
                    (orig.b * 255.0) as u8,
                    mode,
                );
            }
            _ => {
                let col = result.c().clamp();
                emit_char(
                    out,
                    ch,
                    (col.r * 255.0) as u8,
                    (col.g * 255.0) as u8,
                    (col.b * 255.0) as u8,
                    mode,
                );
            }
        },
    }
}

/// Run the compiled shader over text, returning ANSI-colored output
pub fn render(shader: &CompiledShader, text: &str, params: &RenderParams) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut vm = Vm::new(shader);
    let mut out = String::with_capacity(text.len() * 20);
    let lines = parse_ansi_glyph_lines(text);

    if params.cols > 0 {
        let glyphs: Vec<Glyph> = lines.iter().flat_map(|line| line.iter().copied()).collect();
        let len = glyphs.len();
        let cols = params.cols;
        let rows = (len as u32 + cols - 1) / cols;
        for (i, glyph) in glyphs.iter().enumerate() {
            let ch = glyph.ch;
            let col_i = (i as u32 % cols) as f32;
            let row_i = (i as u32 / cols) as f32;
            let t = i as f32 / (len as f32).max(1.0);
            let x = col_i / (cols as f32 - 1.0).max(1.0);
            let y = row_i / (rows as f32 - 1.0).max(1.0);
            let orig = glyph.original;
            let ch_code = ch as u32 as f32;
            let args = [
                Val::F(t),
                Val::F(i as f32),
                Val::F(len as f32),
                Val::F(x),
                Val::F(y),
                Val::F(col_i),
                Val::F(row_i),
                Val::F(ch_code),
                Val::C(orig),
                Val::F(params.time),
            ];
            apply_result(
                &mut out,
                vm.run(shader.entry, &args),
                ch,
                orig,
                params.mode,
                shader.entry_ret,
            );
        }
    } else {
        let rows = lines.len();
        let total: usize = lines.iter().map(|l| l.len()).sum();
        let mut global_i: usize = 0;
        for (row_idx, line) in lines.iter().enumerate() {
            let line_len = line.len();
            for (col_idx, glyph) in line.iter().enumerate() {
                let ch = glyph.ch;
                let col_i = col_idx as f32;
                let row_i = row_idx as f32;
                let t = global_i as f32 / (total as f32).max(1.0);
                let x = col_i / (line_len as f32 - 1.0).max(1.0);
                let y = row_i / (rows as f32 - 1.0).max(1.0);
                let orig = glyph.original;
                let ch_code = ch as u32 as f32;
                let args = [
                    Val::F(t),
                    Val::F(global_i as f32),
                    Val::F(total as f32),
                    Val::F(x),
                    Val::F(y),
                    Val::F(col_i),
                    Val::F(row_i),
                    Val::F(ch_code),
                    Val::C(orig),
                    Val::F(params.time),
                ];
                apply_result(
                    &mut out,
                    vm.run(shader.entry, &args),
                    ch,
                    orig,
                    params.mode,
                    shader.entry_ret,
                );
                global_i += 1;
            }
            if row_idx + 1 < rows {
                out.push('\n');
            }
        }
    }

    if !matches!(params.mode, TextMode::Ascii) {
        out.push_str("\x1b[0m");
    }
    out
}

/// Run shader over text using raw `.ctsl` bytes.
pub fn render_bytes(
    ctsl: &[u8],
    text: &str,
    params: &RenderParams,
) -> Result<String, &'static str> {
    let shader = CompiledShader::from_bytes(ctsl)?;
    Ok(render(&shader, text, params))
}
