use crate::compiler::{CompiledShader, ExternDecl, ExternDefault, ExternType, Op, ReturnType};
use std::collections::HashMap;

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
pub(crate) struct GlyphVal {
    ch: char,
    use_ch: bool,
    fg: Color,
    bg: Color,
    use_bg: bool,
}

#[derive(Clone, Copy)]
pub(crate) enum Val {
    F(f32),
    C(Color),
    V2([f32; 2]),
    V3([f32; 3]),
    V4([f32; 4]),
    M2([[f32; 2]; 2]),
    M3([[f32; 3]; 3]),
    M4([[f32; 4]; 4]),
    Arr(u16),
    G(GlyphVal),
}

impl Val {
    fn f(self) -> f32 {
        match self {
            Val::F(v) => v,
            Val::C(c) => (c.r + c.g + c.b) / 3.0,
            Val::V2(_) | Val::V3(_) | Val::V4(_) | Val::M2(_) | Val::M3(_) | Val::M4(_) => {
                0.0
            }
            Val::Arr(_) => 0.0,
            Val::G(g) => (g.fg.r + g.fg.g + g.fg.b) / 3.0,
        }
    }

    fn c(self) -> Color {
        match self {
            Val::C(c) => c,
            Val::F(v) => Color { r: v, g: v, b: v },
            Val::V2(_) | Val::V3(_) | Val::V4(_) | Val::M2(_) | Val::M3(_) | Val::M4(_) => {
                Color::default()
            }
            Val::Arr(_) => Color::default(),
            Val::G(g) => g.fg,
        }
    }

    fn scalar(self) -> Option<f32> {
        match self {
            Val::F(v) => Some(v),
            _ => None,
        }
    }
}

fn vec_from_slice(values: &[f32]) -> Option<Val> {
    match values {
        [a] => Some(Val::F(*a)),
        [a, b] => Some(Val::V2([*a, *b])),
        [a, b, c] => Some(Val::V3([*a, *b, *c])),
        [a, b, c, d] => Some(Val::V4([*a, *b, *c, *d])),
        _ => None,
    }
}

fn vec_components(v: Val) -> Option<Vec<f32>> {
    match v {
        Val::V2(a) => Some(a.to_vec()),
        Val::V3(a) => Some(a.to_vec()),
        Val::V4(a) => Some(a.to_vec()),
        _ => None,
    }
}

fn swizzle_index(ch: u8) -> Option<usize> {
    match ch {
        b'x' | b'r' | b's' => Some(0),
        b'y' | b'g' | b't' => Some(1),
        b'z' | b'b' | b'p' => Some(2),
        b'w' | b'a' | b'q' => Some(3),
        _ => None,
    }
}

fn swizzle_val(v: Val, mask: &[u8]) -> Option<Val> {
    let mut comps = match v {
        Val::C(c) => vec![c.r, c.g, c.b, 1.0],
        Val::V2(a) => a.to_vec(),
        Val::V3(a) => a.to_vec(),
        Val::V4(a) => a.to_vec(),
        _ => return None,
    };
    if comps.len() < 4 {
        comps.resize(4, 0.0);
    }
    let mut out = Vec::new();
    for &ch in mask {
        let idx = swizzle_index(ch)?;
        out.push(*comps.get(idx)?);
    }
    vec_from_slice(&out)
}

fn componentwise2(a: Val, b: Val, f: fn(f32, f32) -> f32) -> Option<Val> {
    match (a, b) {
        (Val::F(x), Val::F(y)) => Some(Val::F(f(x, y))),
        (Val::C(x), Val::C(y)) => Some(Val::C(Color { r: f(x.r, y.r), g: f(x.g, y.g), b: f(x.b, y.b) })),
        (Val::V2(x), Val::V2(y)) => Some(Val::V2([f(x[0], y[0]), f(x[1], y[1])])),
        (Val::V3(x), Val::V3(y)) => Some(Val::V3([f(x[0], y[0]), f(x[1], y[1]), f(x[2], y[2])])),
        (Val::V4(x), Val::V4(y)) => Some(Val::V4([f(x[0], y[0]), f(x[1], y[1]), f(x[2], y[2]), f(x[3], y[3])])),
        (Val::M2(x), Val::M2(y)) => Some(Val::M2([
            [f(x[0][0], y[0][0]), f(x[0][1], y[0][1])],
            [f(x[1][0], y[1][0]), f(x[1][1], y[1][1])],
        ])),
        (Val::M3(x), Val::M3(y)) => Some(Val::M3([
            [f(x[0][0], y[0][0]), f(x[0][1], y[0][1]), f(x[0][2], y[0][2])],
            [f(x[1][0], y[1][0]), f(x[1][1], y[1][1]), f(x[1][2], y[1][2])],
            [f(x[2][0], y[2][0]), f(x[2][1], y[2][1]), f(x[2][2], y[2][2])],
        ])),
        (Val::M4(x), Val::M4(y)) => Some(Val::M4([
            [f(x[0][0], y[0][0]), f(x[0][1], y[0][1]), f(x[0][2], y[0][2]), f(x[0][3], y[0][3])],
            [f(x[1][0], y[1][0]), f(x[1][1], y[1][1]), f(x[1][2], y[1][2]), f(x[1][3], y[1][3])],
            [f(x[2][0], y[2][0]), f(x[2][1], y[2][1]), f(x[2][2], y[2][2]), f(x[2][3], y[2][3])],
            [f(x[3][0], y[3][0]), f(x[3][1], y[3][1]), f(x[3][2], y[3][2]), f(x[3][3], y[3][3])],
        ])),
        _ => None,
    }
}

fn scale_val(v: Val, s: f32, f: fn(f32, f32) -> f32) -> Option<Val> {
    match v {
        Val::F(x) => Some(Val::F(f(x, s))),
        Val::C(x) => Some(Val::C(Color { r: f(x.r, s), g: f(x.g, s), b: f(x.b, s) })),
        Val::V2(x) => Some(Val::V2([f(x[0], s), f(x[1], s)])),
        Val::V3(x) => Some(Val::V3([f(x[0], s), f(x[1], s), f(x[2], s)])),
        Val::V4(x) => Some(Val::V4([f(x[0], s), f(x[1], s), f(x[2], s), f(x[3], s)])),
        Val::M2(x) => Some(Val::M2([
            [f(x[0][0], s), f(x[0][1], s)],
            [f(x[1][0], s), f(x[1][1], s)],
        ])),
        Val::M3(x) => Some(Val::M3([
            [f(x[0][0], s), f(x[0][1], s), f(x[0][2], s)],
            [f(x[1][0], s), f(x[1][1], s), f(x[1][2], s)],
            [f(x[2][0], s), f(x[2][1], s), f(x[2][2], s)],
        ])),
        Val::M4(x) => Some(Val::M4([
            [f(x[0][0], s), f(x[0][1], s), f(x[0][2], s), f(x[0][3], s)],
            [f(x[1][0], s), f(x[1][1], s), f(x[1][2], s), f(x[1][3], s)],
            [f(x[2][0], s), f(x[2][1], s), f(x[2][2], s), f(x[2][3], s)],
            [f(x[3][0], s), f(x[3][1], s), f(x[3][2], s), f(x[3][3], s)],
        ])),
        _ => None,
    }
}

fn mul_val(a: Val, b: Val) -> Option<Val> {
    match (a, b) {
        (Val::F(x), Val::F(y)) => Some(Val::F(x * y)),
        (Val::C(x), Val::C(y)) => Some(Val::C(Color { r: x.r * y.r, g: x.g * y.g, b: x.b * y.b })),
        (Val::V2(x), Val::V2(y)) => Some(Val::V2([x[0] * y[0], x[1] * y[1]])),
        (Val::V3(x), Val::V3(y)) => Some(Val::V3([x[0] * y[0], x[1] * y[1], x[2] * y[2]])),
        (Val::V4(x), Val::V4(y)) => Some(Val::V4([x[0] * y[0], x[1] * y[1], x[2] * y[2], x[3] * y[3]])),
        (Val::M2(x), Val::M2(y)) => Some(Val::M2([
            [x[0][0] * y[0][0] + x[0][1] * y[1][0], x[0][0] * y[0][1] + x[0][1] * y[1][1]],
            [x[1][0] * y[0][0] + x[1][1] * y[1][0], x[1][0] * y[0][1] + x[1][1] * y[1][1]],
        ])),
        (Val::M3(x), Val::M3(y)) => {
            let mut out = [[0.0; 3]; 3];
            for r in 0..3 {
                for c in 0..3 {
                    out[r][c] = x[r][0] * y[0][c] + x[r][1] * y[1][c] + x[r][2] * y[2][c];
                }
            }
            Some(Val::M3(out))
        }
        (Val::M4(x), Val::M4(y)) => {
            let mut out = [[0.0; 4]; 4];
            for r in 0..4 {
                for c in 0..4 {
                    out[r][c] = x[r][0] * y[0][c] + x[r][1] * y[1][c] + x[r][2] * y[2][c] + x[r][3] * y[3][c];
                }
            }
            Some(Val::M4(out))
        }
        (Val::M2(m), Val::V2(v)) => Some(Val::V2([m[0][0] * v[0] + m[0][1] * v[1], m[1][0] * v[0] + m[1][1] * v[1]])),
        (Val::M3(m), Val::V3(v)) => Some(Val::V3([
            m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
            m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
            m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
        ])),
        (Val::M4(m), Val::V4(v)) => Some(Val::V4([
            m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2] + m[0][3] * v[3],
            m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2] + m[1][3] * v[3],
            m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2] + m[2][3] * v[3],
            m[3][0] * v[0] + m[3][1] * v[1] + m[3][2] * v[2] + m[3][3] * v[3],
        ])),
        (lhs, Val::F(s)) => scale_val(lhs, s, |a, b| a * b),
        (Val::F(s), rhs) => scale_val(rhs, s, |a, b| a * b),
        _ => None,
    }
}

fn div_val(a: Val, b: Val) -> Option<Val> {
    match (a, b) {
        (Val::F(x), Val::F(y)) => Some(Val::F(if y == 0.0 { 0.0 } else { x / y })),
        (Val::C(x), Val::C(y)) => Some(Val::C(Color {
            r: if y.r == 0.0 { 0.0 } else { x.r / y.r },
            g: if y.g == 0.0 { 0.0 } else { x.g / y.g },
            b: if y.b == 0.0 { 0.0 } else { x.b / y.b },
        })),
        (Val::V2(x), Val::V2(y)) => Some(Val::V2([if y[0] == 0.0 { 0.0 } else { x[0] / y[0] }, if y[1] == 0.0 { 0.0 } else { x[1] / y[1] }])),
        (Val::V3(x), Val::V3(y)) => Some(Val::V3([
            if y[0] == 0.0 { 0.0 } else { x[0] / y[0] },
            if y[1] == 0.0 { 0.0 } else { x[1] / y[1] },
            if y[2] == 0.0 { 0.0 } else { x[2] / y[2] },
        ])),
        (Val::V4(x), Val::V4(y)) => Some(Val::V4([
            if y[0] == 0.0 { 0.0 } else { x[0] / y[0] },
            if y[1] == 0.0 { 0.0 } else { x[1] / y[1] },
            if y[2] == 0.0 { 0.0 } else { x[2] / y[2] },
            if y[3] == 0.0 { 0.0 } else { x[3] / y[3] },
        ])),
        (Val::M2(x), Val::M2(y)) => Some(Val::M2([
            [if y[0][0] == 0.0 { 0.0 } else { x[0][0] / y[0][0] }, if y[0][1] == 0.0 { 0.0 } else { x[0][1] / y[0][1] }],
            [if y[1][0] == 0.0 { 0.0 } else { x[1][0] / y[1][0] }, if y[1][1] == 0.0 { 0.0 } else { x[1][1] / y[1][1] }],
        ])),
        (Val::M3(x), Val::M3(y)) => Some(Val::M3([
            [if y[0][0] == 0.0 { 0.0 } else { x[0][0] / y[0][0] }, if y[0][1] == 0.0 { 0.0 } else { x[0][1] / y[0][1] }, if y[0][2] == 0.0 { 0.0 } else { x[0][2] / y[0][2] }],
            [if y[1][0] == 0.0 { 0.0 } else { x[1][0] / y[1][0] }, if y[1][1] == 0.0 { 0.0 } else { x[1][1] / y[1][1] }, if y[1][2] == 0.0 { 0.0 } else { x[1][2] / y[1][2] }],
            [if y[2][0] == 0.0 { 0.0 } else { x[2][0] / y[2][0] }, if y[2][1] == 0.0 { 0.0 } else { x[2][1] / y[2][1] }, if y[2][2] == 0.0 { 0.0 } else { x[2][2] / y[2][2] }],
        ])),
        (Val::M4(x), Val::M4(y)) => Some(Val::M4([
            [if y[0][0] == 0.0 { 0.0 } else { x[0][0] / y[0][0] }, if y[0][1] == 0.0 { 0.0 } else { x[0][1] / y[0][1] }, if y[0][2] == 0.0 { 0.0 } else { x[0][2] / y[0][2] }, if y[0][3] == 0.0 { 0.0 } else { x[0][3] / y[0][3] }],
            [if y[1][0] == 0.0 { 0.0 } else { x[1][0] / y[1][0] }, if y[1][1] == 0.0 { 0.0 } else { x[1][1] / y[1][1] }, if y[1][2] == 0.0 { 0.0 } else { x[1][2] / y[1][2] }, if y[1][3] == 0.0 { 0.0 } else { x[1][3] / y[1][3] }],
            [if y[2][0] == 0.0 { 0.0 } else { x[2][0] / y[2][0] }, if y[2][1] == 0.0 { 0.0 } else { x[2][1] / y[2][1] }, if y[2][2] == 0.0 { 0.0 } else { x[2][2] / y[2][2] }, if y[2][3] == 0.0 { 0.0 } else { x[2][3] / y[2][3] }],
            [if y[3][0] == 0.0 { 0.0 } else { x[3][0] / y[3][0] }, if y[3][1] == 0.0 { 0.0 } else { x[3][1] / y[3][1] }, if y[3][2] == 0.0 { 0.0 } else { x[3][2] / y[3][2] }, if y[3][3] == 0.0 { 0.0 } else { x[3][3] / y[3][3] }],
        ])),
        (lhs, Val::F(s)) => scale_val(lhs, s, |a, b| if b == 0.0 { 0.0 } else { a / b }),
        _ => None,
    }
}

fn neg_val(v: Val) -> Option<Val> {
    scale_val(v, -1.0, |a, b| a * b)
}

fn abs_val(v: Val) -> Option<Val> {
    scale_val(v, 0.0, |a, _| a.abs())
}

fn dot_val(a: Val, b: Val) -> Option<Val> {
    let av = vec_components(a)?;
    let bv = vec_components(b)?;
    if av.len() != bv.len() {
        return None;
    }
    Some(Val::F(av.iter().zip(bv.iter()).map(|(x, y)| x * y).sum()))
}

fn cross_val(a: Val, b: Val) -> Option<Val> {
    let av = vec_components(a)?;
    let bv = vec_components(b)?;
    if av.len() != 3 || bv.len() != 3 {
        return None;
    }
    Some(Val::V3([
        av[1] * bv[2] - av[2] * bv[1],
        av[2] * bv[0] - av[0] * bv[2],
        av[0] * bv[1] - av[1] * bv[0],
    ]))
}

fn normalize_val(v: Val) -> Option<Val> {
    let comps = vec_components(v)?;
    let len = comps.iter().map(|x| x * x).sum::<f32>().sqrt();
    if len == 0.0 {
        return vec_from_slice(&vec![0.0; comps.len()]);
    }
    let out: Vec<f32> = comps.into_iter().map(|x| x / len).collect();
    vec_from_slice(&out)
}

fn reflect_val(i: Val, n: Val) -> Option<Val> {
    let iv = vec_components(i)?;
    let nv = vec_components(n)?;
    if iv.len() != nv.len() {
        return None;
    }
    let dot = iv.iter().zip(nv.iter()).map(|(x, y)| x * y).sum::<f32>();
    let out: Vec<f32> = iv.iter().zip(nv.iter()).map(|(x, y)| x - 2.0 * dot * y).collect();
    vec_from_slice(&out)
}

fn refract_val(i: Val, n: Val, eta: f32) -> Option<Val> {
    let iv = vec_components(i)?;
    let nv = vec_components(n)?;
    if iv.len() != nv.len() {
        return None;
    }
    let dot = iv.iter().zip(nv.iter()).map(|(x, y)| x * y).sum::<f32>();
    let k = 1.0 - eta * eta * (1.0 - dot * dot);
    if k < 0.0 {
        return vec_from_slice(&vec![0.0; iv.len()]);
    }
    let a: Vec<f32> = iv.iter().map(|x| x * eta).collect();
    let b: Vec<f32> = nv.iter().map(|x| x * (eta * dot + k.sqrt())).collect();
    let out: Vec<f32> = a.iter().zip(b.iter()).map(|(x, y)| x - y).collect();
    vec_from_slice(&out)
}

const MAX_LOCALS: usize = 64;
const MAX_STACK: usize = 256;
const MAX_DEPTH: usize = 32;
const MAX_INSTRUCTIONS_PER_RUN: usize = 200_000;

struct Frame {
    fn_idx: usize,
    ip: usize,
    locals: [Val; MAX_LOCALS],
    writeback_len: u8,
    writebacks: [(u8, u8); MAX_LOCALS],
}

impl Frame {
    fn new(fn_idx: usize) -> Self {
        Self {
            fn_idx,
            ip: 0,
            locals: [Val::F(0.0); MAX_LOCALS],
            writeback_len: 0,
            writebacks: [(0, 0); MAX_LOCALS],
        }
    }
}

pub struct Vm<'a> {
    shader: &'a CompiledShader,
    stack: Vec<Val>,
    frames: Vec<Frame>,
    arrays: Vec<Vec<Val>>,
    rng_state: u32,
    last_error: Option<String>,
}

impl<'a> Vm<'a> {
    pub fn new(shader: &'a CompiledShader, seed: u32) -> Self {
        Self {
            shader,
            stack: Vec::with_capacity(MAX_STACK),
            frames: Vec::with_capacity(MAX_DEPTH),
            arrays: Vec::new(),
            rng_state: if seed == 0 { 0x9e3779b9 } else { seed },
            last_error: None,
        }
    }

    #[inline(always)]
    pub fn set_seed(&mut self, seed: u32) {
        self.rng_state = if seed == 0 { 0x9e3779b9 } else { seed };
    }

    #[inline(always)]
    fn next_rand(&mut self) -> f32 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng_state = x;
        (x >> 8) as f32 * (1.0 / 16_777_216.0)
    }

    #[inline(always)]
    fn push(&mut self, v: Val) -> bool {
        if self.stack.len() >= MAX_STACK {
            self.last_error = Some("stack overflow".to_string());
            return false;
        }
        self.stack.push(v);
        true
    }

    #[inline(always)]
    fn pop(&mut self) -> Option<Val> {
        let v = self.stack.pop();
        if v.is_none() {
            self.last_error = Some("stack underflow".to_string());
        }
        v
    }

    fn runtime_fail(&mut self, msg: &str) -> Val {
        if self.last_error.is_none() {
            self.last_error = Some(msg.to_string());
            eprintln!("vm runtime error: {msg}");
        }
        Val::F(0.0)
    }

    fn read_u8(code: &[u8], ip: &mut usize) -> Option<u8> {
        let v = *code.get(*ip)?;
        *ip += 1;
        Some(v)
    }

    fn read_i16(code: &[u8], ip: &mut usize) -> Option<i16> {
        let a = *code.get(*ip)?;
        let b = *code.get(*ip + 1)?;
        *ip += 2;
        Some(i16::from_le_bytes([a, b]))
    }

    fn read_f32(code: &[u8], ip: &mut usize) -> Option<f32> {
        let a = *code.get(*ip)?;
        let b = *code.get(*ip + 1)?;
        let c = *code.get(*ip + 2)?;
        let d = *code.get(*ip + 3)?;
        *ip += 4;
        Some(f32::from_le_bytes([a, b, c, d]))
    }

    pub(crate) fn run(&mut self, fn_idx: usize, args: &[Val]) -> Val {
        self.last_error = None;
        if fn_idx >= self.shader.fns.len() {
            return self.runtime_fail("bad function index");
        }
        if self.shader.fns[fn_idx].code.is_empty() {
            return Val::F(0.0);
        }

        self.frames.clear();
        self.stack.clear();
        self.arrays.clear();

        let mut frame = Frame::new(fn_idx);
        for (i, &a) in args.iter().enumerate().take(MAX_LOCALS) {
            frame.locals[i] = a;
        }

        let mut instruction_count = 0usize;

        loop {
            instruction_count = instruction_count.saturating_add(1);
            if instruction_count > MAX_INSTRUCTIONS_PER_RUN {
                return self.runtime_fail("instruction budget exceeded (possible infinite loop)");
            }

            let f = &self.shader.fns[frame.fn_idx];
            let code = &f.code;
            if frame.ip >= code.len() {
                return self.runtime_fail("instruction pointer out of bounds");
            }

            let op = code[frame.ip];
            frame.ip += 1;

            match op {
                o if o == Op::PushF as u8 => {
                    let Some(v) = Self::read_f32(code, &mut frame.ip) else {
                        return self.runtime_fail("truncated PushF");
                    };
                    if !self.push(Val::F(v)) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::PushC as u8 => {
                    let Some(r) = Self::read_f32(code, &mut frame.ip) else {
                        return self.runtime_fail("truncated PushC r");
                    };
                    let Some(g) = Self::read_f32(code, &mut frame.ip) else {
                        return self.runtime_fail("truncated PushC g");
                    };
                    let Some(b) = Self::read_f32(code, &mut frame.ip) else {
                        return self.runtime_fail("truncated PushC b");
                    };
                    if !self.push(Val::C(Color { r, g, b })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Pop as u8 => {
                    if self.pop().is_none() {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Load as u8 => {
                    let Some(s) = Self::read_u8(code, &mut frame.ip) else {
                        return self.runtime_fail("truncated Load");
                    };
                    let slot = s as usize;
                    if slot >= MAX_LOCALS {
                        return self.runtime_fail("Load local out of bounds");
                    }
                    if !self.push(frame.locals[slot]) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Store as u8 => {
                    let Some(s) = Self::read_u8(code, &mut frame.ip) else {
                        return self.runtime_fail("truncated Store");
                    };
                    let slot = s as usize;
                    if slot >= MAX_LOCALS {
                        return self.runtime_fail("Store local out of bounds");
                    }
                    let Some(v) = self.pop() else {
                        return Val::F(0.0);
                    };
                    frame.locals[slot] = v;
                }
                o if o == Op::AddF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(v) = componentwise2(a, b, |x, y| x + y) else {
                        return Val::F(0.0);
                    };
                    if !self.push(v) { return Val::F(0.0); }
                }
                o if o == Op::SubF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(v) = componentwise2(a, b, |x, y| x - y) else {
                        return Val::F(0.0);
                    };
                    if !self.push(v) { return Val::F(0.0); }
                }
                o if o == Op::MulF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(v) = mul_val(a, b) else {
                        return Val::F(0.0);
                    };
                    if !self.push(v) { return Val::F(0.0); }
                }
                o if o == Op::DivF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(v) = div_val(a, b) else {
                        return Val::F(0.0);
                    };
                    if !self.push(v) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::ModF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(v) = componentwise2(a, b, |x, y| x % y) else {
                        return Val::F(0.0);
                    };
                    if !self.push(v) { return Val::F(0.0); }
                }
                o if o == Op::NegF as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(v) = neg_val(a) else {
                        return Val::F(0.0);
                    };
                    if !self.push(v) { return Val::F(0.0); }
                }
                o if o == Op::AbsF as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(v) = abs_val(a) else {
                        return Val::F(0.0);
                    };
                    if !self.push(v) { return Val::F(0.0); }
                }
                o if o == Op::AddC as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let b = b.c();
                    let a = a.c();
                    if !self.push(Val::C(Color {
                        r: a.r + b.r,
                        g: a.g + b.g,
                        b: a.b + b.b,
                    })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::SubC as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let b = b.c();
                    let a = a.c();
                    if !self.push(Val::C(Color {
                        r: a.r - b.r,
                        g: a.g - b.g,
                        b: a.b - b.b,
                    })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::MulC as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let b = b.c();
                    let a = a.c();
                    if !self.push(Val::C(Color {
                        r: a.r * b.r,
                        g: a.g * b.g,
                        b: a.b * b.b,
                    })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::MulCF as u8 => {
                    let Some(t) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let t = t.f();
                    let a = a.c();
                    if !self.push(Val::C(Color {
                        r: a.r * t,
                        g: a.g * t,
                        b: a.b * t,
                    })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::DivC as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let b = b.c();
                    let a = a.c();
                    let r = if b.r == 0.0 { 0.0 } else { a.r / b.r };
                    let g = if b.g == 0.0 { 0.0 } else { a.g / b.g };
                    let bv = if b.b == 0.0 { 0.0 } else { a.b / b.b };
                    if !self.push(Val::C(Color { r, g, b: bv })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::EqF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(if a.f() == b.f() { 1.0 } else { 0.0 })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::NeF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(if a.f() != b.f() { 1.0 } else { 0.0 })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::LtF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(if a.f() < b.f() { 1.0 } else { 0.0 })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::GtF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(if a.f() > b.f() { 1.0 } else { 0.0 })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::LeF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(if a.f() <= b.f() { 1.0 } else { 0.0 })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::GeF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(if a.f() >= b.f() { 1.0 } else { 0.0 })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::AndF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(if a.f() != 0.0 && b.f() != 0.0 {
                        1.0
                    } else {
                        0.0
                    })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::OrF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(if a.f() != 0.0 || b.f() != 0.0 {
                        1.0
                    } else {
                        0.0
                    })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::NotF as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(if a.f() == 0.0 { 1.0 } else { 0.0 })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Sin as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().sin())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Cos as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().cos())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Tan as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().tan())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Asin as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().asin())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Acos as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().acos())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Atan as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().atan())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Atan2 as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().atan2(b.f()))) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Sqrt as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().sqrt())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Pow as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().powf(b.f()))) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Exp as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().exp())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Log as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().ln())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Log2 as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().log2())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Floor as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().floor())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Ceil as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().ceil())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Round as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().round())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Fract as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().fract())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Min2 as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().min(b.f()))) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Max2 as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().max(b.f()))) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Clamp as u8 => {
                    let Some(hi) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(lo) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().clamp(lo.f(), hi.f()))) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Mix as u8 => {
                    let Some(t) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    match (a, b) {
                        (Val::C(ca), Val::C(cb)) => {
                            let tf = t.f();
                            if !self.push(Val::C(Color {
                                r: ca.r + (cb.r - ca.r) * tf,
                                g: ca.g + (cb.g - ca.g) * tf,
                                b: ca.b + (cb.b - ca.b) * tf,
                            })) {
                                return Val::F(0.0);
                            }
                        }
                        _ => {
                            let tf = t.f();
                            if !self.push(Val::F(a.f() + (b.f() - a.f()) * tf)) {
                                return Val::F(0.0);
                            }
                        }
                    }
                }
                o if o == Op::Step as u8 => {
                    let Some(x) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(e) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(if x.f() < e.f() { 0.0 } else { 1.0 })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Smoothstep as u8 => {
                    let Some(x) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(e1) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(e0) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let denom = e1.f() - e0.f();
                    let t = if denom == 0.0 {
                        0.0
                    } else {
                        ((x.f() - e0.f()) / denom).clamp(0.0, 1.0)
                    };
                    if !self.push(Val::F(t * t * (3.0 - 2.0 * t))) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Sign as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().signum())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Length2 as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let out = match (a, b) {
                        (Val::F(x), Val::F(y)) => Val::F((x * x + y * y).sqrt()),
                        _ => match dot_val(a, b) {
                            Some(v) => Val::F(v.f().sqrt()),
                            None => return Val::F(0.0),
                        },
                    };
                    if !self.push(out) { return Val::F(0.0); }
                }
                o if o == Op::Rgb as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(g) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(r) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::C(Color {
                        r: r.f() / 255.0,
                        g: g.f() / 255.0,
                        b: b.f() / 255.0,
                    })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Rgba as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(g) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(r) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let mut af = a.f();
                    if af > 1.0 {
                        af /= 255.0;
                    }
                    af = af.clamp(0.0, 1.0);
                    if !self.push(Val::C(Color {
                        r: (r.f() / 255.0) * af,
                        g: (g.f() / 255.0) * af,
                        b: (b.f() / 255.0) * af,
                    })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Hsl as u8 => {
                    let Some(l) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(s) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(h) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::C(hsl_to_rgb(h.f(), s.f(), l.f()))) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Hsv as u8 => {
                    let Some(v) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(s) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(h) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::C(hsv_to_rgb(h.f(), s.f(), v.f()))) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Gray as u8 => {
                    let Some(v) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let vf = v.f();
                    if !self.push(Val::C(Color {
                        r: vf,
                        g: vf,
                        b: vf,
                    })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Mix2C as u8 => {
                    let Some(t) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let t = t.f();
                    let b = b.c();
                    let a = a.c();
                    if !self.push(Val::C(Color {
                        r: a.r + (b.r - a.r) * t,
                        g: a.g + (b.g - a.g) * t,
                        b: a.b + (b.b - a.b) * t,
                    })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::GetR as u8 => {
                    let Some(c) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(c.c().r)) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::GetG as u8 => {
                    let Some(c) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(c.c().g)) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::GetB as u8 => {
                    let Some(c) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(c.c().b)) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::IsSpace as u8 => {
                    let Some(code_v) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let code = code_v.f() as u32;
                    let ch = char::from_u32(code).unwrap_or('\0');
                    if !self.push(Val::F(if ch.is_whitespace() { 1.0 } else { 0.0 })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::IsDigit as u8 => {
                    let Some(code_v) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let code = code_v.f() as u32;
                    let ch = char::from_u32(code).unwrap_or('\0');
                    if !self.push(Val::F(if ch.is_ascii_digit() { 1.0 } else { 0.0 })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::IsAlpha as u8 => {
                    let Some(code_v) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let code = code_v.f() as u32;
                    let ch = char::from_u32(code).unwrap_or('\0');
                    if !self.push(Val::F(if ch.is_ascii_alphabetic() { 1.0 } else { 0.0 })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::IsUpper as u8 => {
                    let Some(code_v) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let code = code_v.f() as u32;
                    let ch = char::from_u32(code).unwrap_or('\0');
                    if !self.push(Val::F(if ch.is_ascii_uppercase() { 1.0 } else { 0.0 })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::IsLower as u8 => {
                    let Some(code_v) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let code = code_v.f() as u32;
                    let ch = char::from_u32(code).unwrap_or('\0');
                    if !self.push(Val::F(if ch.is_ascii_lowercase() { 1.0 } else { 0.0 })) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Jmp as u8 => {
                    let Some(off) = Self::read_i16(code, &mut frame.ip) else {
                        return self.runtime_fail("truncated Jmp");
                    };
                    let target = frame.ip as i64 + off as i64;
                    if target < 0 || target as usize > code.len() {
                        return self.runtime_fail("jump target out of bounds");
                    }
                    frame.ip = target as usize;
                }
                o if o == Op::JmpZ as u8 => {
                    let Some(off) = Self::read_i16(code, &mut frame.ip) else {
                        return self.runtime_fail("truncated JmpZ");
                    };
                    let Some(v) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if v.f() == 0.0 {
                        let target = frame.ip as i64 + off as i64;
                        if target < 0 || target as usize > code.len() {
                            return self.runtime_fail("jump target out of bounds");
                        }
                        frame.ip = target as usize;
                    }
                }
                o if o == Op::Ret as u8 => {
                    let ret = self.pop().unwrap_or(Val::F(0.0));
                    if let Some(mut parent) = self.frames.pop() {
                        for i in 0..(frame.writeback_len as usize).min(MAX_LOCALS) {
                            let (callee_slot, caller_slot) = frame.writebacks[i];
                            let callee_idx = callee_slot as usize;
                            let caller_idx = caller_slot as usize;
                            if callee_idx < MAX_LOCALS && caller_idx < MAX_LOCALS {
                                parent.locals[caller_idx] = frame.locals[callee_idx];
                            }
                        }
                        frame = parent;
                        if !self.push(ret) {
                            return Val::F(0.0);
                        }
                    } else {
                        return ret;
                    }
                }
                o if o == Op::Call as u8 => {
                    let Some(idx_b) = Self::read_u8(code, &mut frame.ip) else {
                        return self.runtime_fail("truncated Call fn index");
                    };
                    let Some(argc_b) = Self::read_u8(code, &mut frame.ip) else {
                        return self.runtime_fail("truncated Call argc");
                    };
                    let idx = idx_b as usize;
                    let argc = argc_b as usize;
                    if idx >= self.shader.fns.len() {
                        return self.runtime_fail("call target out of range");
                    }
                    if argc > self.stack.len() {
                        return self.runtime_fail("call argc exceeds stack size");
                    }
                    if self.frames.len() >= MAX_DEPTH {
                        return self.runtime_fail("call depth exceeded");
                    }

                    let mut new_frame = Frame::new(idx);
                    let base = self.stack.len() - argc;
                    for i in 0..argc.min(MAX_LOCALS) {
                        new_frame.locals[i] = self.stack[base + i];
                    }
                    self.stack.truncate(base);
                    let old = std::mem::replace(&mut frame, new_frame);
                    self.frames.push(old);
                }
                o if o == Op::CallExt as u8 => {
                    let Some(idx_b) = Self::read_u8(code, &mut frame.ip) else {
                        return self.runtime_fail("truncated CallExt fn index");
                    };
                    let Some(argc_b) = Self::read_u8(code, &mut frame.ip) else {
                        return self.runtime_fail("truncated CallExt argc");
                    };
                    let Some(wcount_b) = Self::read_u8(code, &mut frame.ip) else {
                        return self.runtime_fail("truncated CallExt writeback count");
                    };
                    let idx = idx_b as usize;
                    let argc = argc_b as usize;
                    let wcount = wcount_b as usize;
                    if idx >= self.shader.fns.len() {
                        return self.runtime_fail("call target out of range");
                    }
                    if argc > self.stack.len() {
                        return self.runtime_fail("call argc exceeds stack size");
                    }
                    if self.frames.len() >= MAX_DEPTH {
                        return self.runtime_fail("call depth exceeded");
                    }
                    if wcount > MAX_LOCALS {
                        return self.runtime_fail("CallExt writeback count exceeds local capacity");
                    }

                    let mut new_frame = Frame::new(idx);
                    let base = self.stack.len() - argc;
                    for i in 0..argc.min(MAX_LOCALS) {
                        new_frame.locals[i] = self.stack[base + i];
                    }
                    new_frame.writeback_len = wcount as u8;
                    for i in 0..wcount {
                        let Some(callee_slot) = Self::read_u8(code, &mut frame.ip) else {
                            return self.runtime_fail("truncated CallExt callee slot");
                        };
                        let Some(caller_slot) = Self::read_u8(code, &mut frame.ip) else {
                            return self.runtime_fail("truncated CallExt caller slot");
                        };
                        new_frame.writebacks[i] = (callee_slot, caller_slot);
                    }
                    self.stack.truncate(base);
                    let old = std::mem::replace(&mut frame, new_frame);
                    self.frames.push(old);
                }
                o if o == Op::Rand as u8 => {
                    let rv = self.next_rand();
                    if !self.push(Val::F(rv)) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::RandBetween as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let af = a.f();
                    let bf = b.f();
                    let rv = self.next_rand();
                    if !self.push(Val::F(af + rv * (bf - af))) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::MakeGlyphCharFg as u8 => {
                    let Some(fg) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(ch) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let glyph = GlyphVal {
                        ch: char::from_u32(ch.f() as u32).unwrap_or('\0'),
                        use_ch: true,
                        fg: fg.c(),
                        bg: Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                        },
                        use_bg: false,
                    };
                    if !self.push(Val::G(glyph)) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::MakeGlyphFgBg as u8 => {
                    let Some(bg) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(fg) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let glyph = GlyphVal {
                        ch: '\0',
                        use_ch: false,
                        fg: fg.c(),
                        bg: bg.c(),
                        use_bg: true,
                    };
                    if !self.push(Val::G(glyph)) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::MakeGlyphCharFgBg as u8 => {
                    let Some(bg) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(fg) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(ch) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let glyph = GlyphVal {
                        ch: char::from_u32(ch.f() as u32).unwrap_or('\0'),
                        use_ch: true,
                        fg: fg.c(),
                        bg: bg.c(),
                        use_bg: true,
                    };
                    if !self.push(Val::G(glyph)) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::HashF as u8 => {
                    let Some(s) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let v =
                        (s.f() * 127.1_f32 + self.rng_state as f32 * 0.0001).sin() * 43758.5453_f32;
                    if !self.push(Val::F(v - v.floor())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::Vec2 as u8 => {
                    let Some(y) = self.pop().and_then(|v| v.scalar()) else { return Val::F(0.0); };
                    let Some(x) = self.pop().and_then(|v| v.scalar()) else { return Val::F(0.0); };
                    if !self.push(Val::V2([x, y])) { return Val::F(0.0); }
                }
                o if o == Op::Vec3 as u8 => {
                    let Some(z) = self.pop().and_then(|v| v.scalar()) else { return Val::F(0.0); };
                    let Some(y) = self.pop().and_then(|v| v.scalar()) else { return Val::F(0.0); };
                    let Some(x) = self.pop().and_then(|v| v.scalar()) else { return Val::F(0.0); };
                    if !self.push(Val::V3([x, y, z])) { return Val::F(0.0); }
                }
                o if o == Op::Vec4 as u8 => {
                    let Some(w) = self.pop().and_then(|v| v.scalar()) else { return Val::F(0.0); };
                    let Some(z) = self.pop().and_then(|v| v.scalar()) else { return Val::F(0.0); };
                    let Some(y) = self.pop().and_then(|v| v.scalar()) else { return Val::F(0.0); };
                    let Some(x) = self.pop().and_then(|v| v.scalar()) else { return Val::F(0.0); };
                    if !self.push(Val::V4([x, y, z, w])) { return Val::F(0.0); }
                }
                o if o == Op::Mat2 as u8 => {
                    let mut vals = [0.0; 4];
                    for idx in (0..4).rev() {
                        let Some(v) = self.pop().and_then(|v| v.scalar()) else { return Val::F(0.0); };
                        vals[idx] = v;
                    }
                    if !self.push(Val::M2([[vals[0], vals[1]], [vals[2], vals[3]]])) { return Val::F(0.0); }
                }
                o if o == Op::Mat3 as u8 => {
                    let mut vals = [0.0; 9];
                    for idx in (0..9).rev() {
                        let Some(v) = self.pop().and_then(|v| v.scalar()) else { return Val::F(0.0); };
                        vals[idx] = v;
                    }
                    if !self.push(Val::M3([
                        [vals[0], vals[1], vals[2]],
                        [vals[3], vals[4], vals[5]],
                        [vals[6], vals[7], vals[8]],
                    ])) { return Val::F(0.0); }
                }
                o if o == Op::Mat4 as u8 => {
                    let mut vals = [0.0; 16];
                    for idx in (0..16).rev() {
                        let Some(v) = self.pop().and_then(|v| v.scalar()) else { return Val::F(0.0); };
                        vals[idx] = v;
                    }
                    if !self.push(Val::M4([
                        [vals[0], vals[1], vals[2], vals[3]],
                        [vals[4], vals[5], vals[6], vals[7]],
                        [vals[8], vals[9], vals[10], vals[11]],
                        [vals[12], vals[13], vals[14], vals[15]],
                    ])) { return Val::F(0.0); }
                }
                o if o == Op::Swizzle as u8 => {
                    let Some(len) = Self::read_u8(code, &mut frame.ip) else { return self.runtime_fail("truncated Swizzle"); };
                    let mut mask = vec![0u8; len as usize];
                    for slot in &mut mask {
                        let Some(ch) = Self::read_u8(code, &mut frame.ip) else { return self.runtime_fail("truncated Swizzle mask"); };
                        *slot = ch;
                    }
                    let Some(v) = self.pop() else { return Val::F(0.0); };
                    let Some(outv) = swizzle_val(v, &mask) else { return Val::F(0.0); };
                    if !self.push(outv) { return Val::F(0.0); }
                }
                o if o == Op::Dot as u8 => {
                    let Some(b) = self.pop() else { return Val::F(0.0); };
                    let Some(a) = self.pop() else { return Val::F(0.0); };
                    let Some(v) = dot_val(a, b) else { return Val::F(0.0); };
                    if !self.push(v) { return Val::F(0.0); }
                }
                o if o == Op::Cross as u8 => {
                    let Some(b) = self.pop() else { return Val::F(0.0); };
                    let Some(a) = self.pop() else { return Val::F(0.0); };
                    let Some(v) = cross_val(a, b) else { return Val::F(0.0); };
                    if !self.push(v) { return Val::F(0.0); }
                }
                o if o == Op::Normalize as u8 => {
                    let Some(a) = self.pop() else { return Val::F(0.0); };
                    let Some(v) = normalize_val(a) else { return Val::F(0.0); };
                    if !self.push(v) { return Val::F(0.0); }
                }
                o if o == Op::Reflect as u8 => {
                    let Some(n) = self.pop() else { return Val::F(0.0); };
                    let Some(i) = self.pop() else { return Val::F(0.0); };
                    let Some(v) = reflect_val(i, n) else { return Val::F(0.0); };
                    if !self.push(v) { return Val::F(0.0); }
                }
                o if o == Op::Refract as u8 => {
                    let Some(eta) = self.pop().and_then(|v| v.scalar()) else { return Val::F(0.0); };
                    let Some(n) = self.pop() else { return Val::F(0.0); };
                    let Some(i) = self.pop() else { return Val::F(0.0); };
                    let Some(v) = refract_val(i, n, eta) else { return Val::F(0.0); };
                    if !self.push(v) { return Val::F(0.0); }
                }
                o if o == Op::ArrayMake as u8 => {
                    let Some(len) = Self::read_u8(code, &mut frame.ip) else {
                        return self.runtime_fail("truncated ArrayMake");
                    };
                    let mut values = Vec::with_capacity(len as usize);
                    for _ in 0..len {
                        let Some(v) = self.pop() else {
                            return Val::F(0.0);
                        };
                        values.push(v);
                    }
                    values.reverse();
                    if self.arrays.len() >= u16::MAX as usize {
                        return self.runtime_fail("array heap overflow");
                    }
                    let handle = self.arrays.len() as u16;
                    self.arrays.push(values);
                    if !self.push(Val::Arr(handle)) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::ArrayGet as u8 => {
                    let Some(idx_raw) = self.pop().and_then(|v| v.scalar()) else { return Val::F(0.0); };
                    let Some(arr) = self.pop() else { return Val::F(0.0); };
                    let Val::Arr(handle) = arr else {
                        return Val::F(0.0);
                    };
                    let Some(items) = self.arrays.get(handle as usize) else {
                        return Val::F(0.0);
                    };
                    if items.is_empty() {
                        if !self.push(Val::F(0.0)) {
                            return Val::F(0.0);
                        }
                        continue;
                    }
                    let max_idx = (items.len() - 1) as f32;
                    let clamped = idx_raw.clamp(0.0, max_idx).trunc() as usize;
                    if !self.push(items[clamped]) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::ArraySet as u8 => {
                    let Some(value) = self.pop() else { return Val::F(0.0); };
                    let Some(idx_raw) = self.pop().and_then(|v| v.scalar()) else { return Val::F(0.0); };
                    let Some(arr) = self.pop() else { return Val::F(0.0); };
                    let Val::Arr(handle) = arr else {
                        return Val::F(0.0);
                    };
                    let Some(items) = self.arrays.get_mut(handle as usize) else {
                        return Val::F(0.0);
                    };
                    if items.is_empty() {
                        continue;
                    }
                    let max_idx = (items.len() - 1) as f32;
                    let clamped = idx_raw.clamp(0.0, max_idx).trunc() as usize;
                    items[clamped] = value;
                }
                _ => {
                    return self.runtime_fail("unknown opcode");
                }
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

#[derive(Debug, Clone, Copy)]
pub enum ExternValue {
    Number(f32),
    Color(Color),
    Char(char),
    Bool(bool),
    CharFg(char, Color),
    FgBg(Color, Color),
    CharFgBg(char, Color, Color),
}

impl From<f32> for ExternValue {
    fn from(value: f32) -> Self {
        ExternValue::Number(value)
    }
}

impl From<f64> for ExternValue {
    fn from(value: f64) -> Self {
        ExternValue::Number(value as f32)
    }
}

impl From<i32> for ExternValue {
    fn from(value: i32) -> Self {
        ExternValue::Number(value as f32)
    }
}

impl From<u32> for ExternValue {
    fn from(value: u32) -> Self {
        ExternValue::Number(value as f32)
    }
}

impl From<usize> for ExternValue {
    fn from(value: usize) -> Self {
        ExternValue::Number(value as f32)
    }
}

impl From<isize> for ExternValue {
    fn from(value: isize) -> Self {
        ExternValue::Number(value as f32)
    }
}

impl From<bool> for ExternValue {
    fn from(value: bool) -> Self {
        ExternValue::Bool(value)
    }
}

impl From<char> for ExternValue {
    fn from(value: char) -> Self {
        ExternValue::Char(value)
    }
}

impl From<Color> for ExternValue {
    fn from(value: Color) -> Self {
        ExternValue::Color(value)
    }
}

impl From<[f32; 3]> for ExternValue {
    fn from(value: [f32; 3]) -> Self {
        ExternValue::Color(Color {
            r: value[0],
            g: value[1],
            b: value[2],
        })
    }
}

impl From<(u8, u8, u8)> for ExternValue {
    fn from(value: (u8, u8, u8)) -> Self {
        ExternValue::Color(Color {
            r: value.0 as f32 / 255.0,
            g: value.1 as f32 / 255.0,
            b: value.2 as f32 / 255.0,
        })
    }
}

impl From<(char, Color)> for ExternValue {
    fn from(value: (char, Color)) -> Self {
        ExternValue::CharFg(value.0, value.1)
    }
}

impl From<(Color, Color)> for ExternValue {
    fn from(value: (Color, Color)) -> Self {
        ExternValue::FgBg(value.0, value.1)
    }
}

impl From<(char, Color, Color)> for ExternValue {
    fn from(value: (char, Color, Color)) -> Self {
        ExternValue::CharFgBg(value.0, value.1, value.2)
    }
}

#[macro_export]
macro_rules! externs {
    () => {
        externs: ::std::collections::HashMap::new()
    };
    ($($name:literal : $value:expr),+ $(,)?) => {
        externs: {
            let mut __tsl_externs = ::std::collections::HashMap::new();
            $(
                __tsl_externs.insert(($name).to_string(), $crate::ExternValue::from($value));
            )+
            __tsl_externs
        }
    };
}

fn color_from_arr(c: [f32; 3]) -> Color {
    Color {
        r: c[0],
        g: c[1],
        b: c[2],
    }
}

fn extern_default_to_val(def: &ExternDefault) -> Val {
    match def {
        ExternDefault::Number(v) => Val::F(*v),
        ExternDefault::Color(c) => Val::C(color_from_arr(*c)),
        ExternDefault::Char(ch) => Val::F(*ch as u32 as f32),
        ExternDefault::Bool(v) => Val::F(if *v { 1.0 } else { 0.0 }),
        ExternDefault::CharFg(ch, fg) => Val::G(GlyphVal {
            ch: *ch,
            use_ch: true,
            fg: color_from_arr(*fg),
            bg: Color::default(),
            use_bg: false,
        }),
        ExternDefault::FgBg(fg, bg) => Val::G(GlyphVal {
            ch: '\0',
            use_ch: false,
            fg: color_from_arr(*fg),
            bg: color_from_arr(*bg),
            use_bg: true,
        }),
        ExternDefault::CharFgBg(ch, fg, bg) => Val::G(GlyphVal {
            ch: *ch,
            use_ch: true,
            fg: color_from_arr(*fg),
            bg: color_from_arr(*bg),
            use_bg: true,
        }),
    }
}

fn extern_value_to_val(name: &str, ty: ExternType, val: ExternValue) -> Result<Val, String> {
    match ty {
        ExternType::Number => match val {
            ExternValue::Number(v) => Ok(Val::F(v)),
            _ => Err(format!("extern '{}' expects number", name)),
        },
        ExternType::Color => match val {
            ExternValue::Color(c) => Ok(Val::C(c)),
            _ => Err(format!("extern '{}' expects color", name)),
        },
        ExternType::Char => match val {
            ExternValue::Char(ch) => Ok(Val::F(ch as u32 as f32)),
            ExternValue::Number(v) => {
                let code = v as u32;
                Ok(Val::F(char::from_u32(code).unwrap_or('\0') as u32 as f32))
            }
            _ => Err(format!("extern '{}' expects char", name)),
        },
        ExternType::Bool => match val {
            ExternValue::Bool(v) => Ok(Val::F(if v { 1.0 } else { 0.0 })),
            _ => Err(format!("extern '{}' expects bool", name)),
        },
        ExternType::CharFg => match val {
            ExternValue::CharFg(ch, fg) => Ok(Val::G(GlyphVal {
                ch,
                use_ch: true,
                fg,
                bg: Color::default(),
                use_bg: false,
            })),
            _ => Err(format!("extern '{}' expects (char,color)", name)),
        },
        ExternType::FgBg => match val {
            ExternValue::FgBg(fg, bg) => Ok(Val::G(GlyphVal {
                ch: '\0',
                use_ch: false,
                fg,
                bg,
                use_bg: true,
            })),
            _ => Err(format!("extern '{}' expects (color,color)", name)),
        },
        ExternType::CharFgBg => match val {
            ExternValue::CharFgBg(ch, fg, bg) => Ok(Val::G(GlyphVal {
                ch,
                use_ch: true,
                fg,
                bg,
                use_bg: true,
            })),
            _ => Err(format!("extern '{}' expects (char,color,color)", name)),
        },
    }
}

fn format_extern_default(def: &ExternDefault) -> String {
    fn channel_to_u8(value: f32) -> u8 {
        (value * 255.0).round().clamp(0.0, 255.0) as u8
    }

    fn format_rgb(color: [f32; 3]) -> String {
        format!(
            "rgb({}, {}, {})",
            channel_to_u8(color[0]),
            channel_to_u8(color[1]),
            channel_to_u8(color[2])
        )
    }

    match def {
        ExternDefault::Number(v) => format!("{}", v),
        ExternDefault::Color(c) => format_rgb(*c),
        ExternDefault::Char(ch) => format!("'{}'", ch),
        ExternDefault::Bool(v) => format!("{}", v),
        ExternDefault::CharFg(ch, fg) => {
            format!("('{}', {})", ch, format_rgb(*fg))
        }
        ExternDefault::FgBg(fg, bg) => format!(
            "({}, {})",
            format_rgb(*fg),
            format_rgb(*bg)
        ),
        ExternDefault::CharFgBg(ch, fg, bg) => format!(
            "('{}', {}, {})",
            ch,
            format_rgb(*fg),
            format_rgb(*bg)
        ),
    }
}

fn resolve_extern_arg(ext: &ExternDecl, provided: Option<&ExternValue>) -> Result<Val, String> {
    if let Some(v) = provided {
        return extern_value_to_val(&ext.name, ext.ty, *v);
    }
    if let Some(def) = &ext.default {
        return Ok(extern_default_to_val(def));
    }
    Err(format!(
        "missing required extern '{}' (type {:?})",
        ext.name, ext.ty
    ))
}

fn resolve_extern_args(
    shader: &CompiledShader,
    externs: &HashMap<String, ExternValue>,
) -> Result<Vec<Val>, String> {
    let mut out = Vec::with_capacity(shader.externs.len());
    for ext in &shader.externs {
        let resolved = resolve_extern_arg(ext, externs.get(&ext.name))?;
        out.push(resolved);
    }
    Ok(out)
}

pub struct RenderParams {
    pub mode: TextMode,
    pub charset: CharSet,
    pub cols: u32,
    pub time: f32,
    pub seed: u32,
    pub externs: HashMap<String, ExternValue>,
}

impl Default for RenderParams {
    fn default() -> Self {
        Self {
            mode: TextMode::Ansi24,
            charset: CharSet::Ascii,
            cols: 0,
            time: 0.0,
            seed: 0x9e3779b9,
            externs: HashMap::default(),
        }
    }
}

#[derive(Clone, Copy)]
struct Glyph {
    ch: char,
    original_fg: Color,
    original_bg: Color,
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

fn apply_sgr(seq: &str, fg: &mut Color, bg: &mut Color) {
    let src = if seq.is_empty() { "0" } else { seq };
    let mut it = src.split(';').map(|s| s.parse::<u16>().unwrap_or(0));
    while let Some(code) = it.next() {
        match code {
            0 => {
                *fg = Color {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                };
                *bg = Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                };
            }
            39 => {
                *fg = Color {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                };
            }
            49 => {
                *bg = Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                };
            }
            30..=37 => *fg = color_from_ansi16((code - 30) as u8),
            40..=47 => *bg = color_from_ansi16((code - 40) as u8),
            90..=97 => *fg = color_from_ansi16((code - 90 + 8) as u8),
            100..=107 => *bg = color_from_ansi16((code - 100 + 8) as u8),
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
            48 => match it.next() {
                Some(5) => {
                    if let Some(n) = it.next() {
                        *bg = color_from_xterm256(n as u8);
                    }
                }
                Some(2) => {
                    if let (Some(r), Some(g), Some(b)) = (it.next(), it.next(), it.next()) {
                        *bg = Color {
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
        let black = Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        };
        return text
            .split('\n')
            .map(|line| {
                line.chars()
                    .map(|ch| Glyph {
                        ch,
                        original_fg: white,
                        original_bg: black,
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
    let mut bg = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
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
                    apply_sgr(&text[i + 2..j], &mut fg, &mut bg);
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
            } else if let Some(line) = lines.last_mut() {
                line.push(Glyph {
                    ch,
                    original_fg: fg,
                    original_bg: bg,
                });
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

fn emit_char(out: &mut String, ch: char, fg: Color, bg: Option<Color>, mode: TextMode) {
    let fg = fg.clamp();
    let fr = (fg.r * 255.0) as u8;
    let fgc = (fg.g * 255.0) as u8;
    let fb = (fg.b * 255.0) as u8;
    let (br, bgc, bb, has_bg) = if let Some(bg) = bg {
        let bg = bg.clamp();
        (
            (bg.r * 255.0) as u8,
            (bg.g * 255.0) as u8,
            (bg.b * 255.0) as u8,
            true,
        )
    } else {
        (0, 0, 0, false)
    };

    match mode {
        TextMode::Ascii => out.push(ch),
        TextMode::Ansi8 => {
            out.push_str("\x1b[38;5;");
            push_dec(out, crate::util::ansi8_color(fr, fgc, fb));
            if has_bg {
                out.push_str("m\x1b[48;5;");
                push_dec(out, crate::util::ansi8_color(br, bgc, bb));
                out.push('m');
            } else {
                out.push_str("m\x1b[49m");
            }
            out.push(ch);
        }
        TextMode::Ansi24 => {
            out.push_str("\x1b[38;2;");
            push_dec(out, fr);
            out.push(';');
            push_dec(out, fgc);
            out.push(';');
            push_dec(out, fb);
            if has_bg {
                out.push_str("m\x1b[48;2;");
                push_dec(out, br);
                out.push(';');
                push_dec(out, bgc);
                out.push(';');
                push_dec(out, bb);
                out.push('m');
            } else {
                out.push_str("m\x1b[49m");
            }
            out.push(ch);
        }
    }
}

#[inline(always)]
fn apply_result(
    out: &mut String,
    result: Val,
    ch: char,
    orig_fg: Color,
    _orig_bg: Color,
    mode: TextMode,
    ret: ReturnType,
) {
    match result {
        Val::G(g) => {
            let out_ch = if g.use_ch { g.ch } else { ch };
            let bg = if g.use_bg { Some(g.bg) } else { None };
            emit_char(out, out_ch, g.fg, bg, mode);
        }
        _ => match ret {
            ReturnType::Char => {
                let new_ch = char::from_u32(result.f() as u32).unwrap_or(ch);
                emit_char(out, new_ch, orig_fg, None, mode);
            }
            ReturnType::Color => {
                emit_char(out, ch, result.c(), None, mode);
            }
            ReturnType::CharFg => {
                let new_ch = char::from_u32(result.f() as u32).unwrap_or(ch);
                emit_char(out, new_ch, orig_fg, None, mode);
            }
            ReturnType::FgBg => {
                emit_char(out, ch, result.c(), None, mode);
            }
            ReturnType::CharFgBg => {
                let new_ch = char::from_u32(result.f() as u32).unwrap_or(ch);
                emit_char(out, new_ch, orig_fg, None, mode);
            }
        },
    }
}

#[inline(always)]
fn mix_seed(base: u32, idx: u32, t_bits: u32) -> u32 {
    let mut x = base ^ idx.wrapping_mul(0x9e3779b9) ^ t_bits.rotate_left(13);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846ca68b);
    x ^ (x >> 16)
}

/// Run the compiled shader over text, returning ANSI-colored output
pub fn render(
    shader: &CompiledShader,
    text: &str,
    params: &RenderParams,
) -> Result<String, String> {
    if text.is_empty() {
        return Ok(String::new());
    }
    if shader.fns.is_empty() || shader.entry >= shader.fns.len() {
        return Ok(text.to_string());
    }

    let extern_args = resolve_extern_args(shader, &params.externs)?;

    let mut vm = Vm::new(shader, params.seed);
    let mut out = String::with_capacity(text.len() * 24);
    let lines = parse_ansi_glyph_lines(text);

    if params.cols > 0 {
        let glyphs: Vec<Glyph> = lines.iter().flat_map(|line| line.iter().copied()).collect();
        let len = glyphs.len();
        let cols = params.cols;
        let rows = (len as u32).div_ceil(cols);
        for (i, glyph) in glyphs.iter().enumerate() {
            let ch = glyph.ch;
            let col_i = (i as u32 % cols) as f32;
            let row_i = (i as u32 / cols) as f32;
            let t = i as f32 / (len as f32).max(1.0);
            let x = col_i / (cols as f32 - 1.0).max(1.0);
            let y = row_i / (rows as f32 - 1.0).max(1.0);
            let orig_fg = glyph.original_fg;
            let orig_bg = glyph.original_bg;
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
                Val::C(orig_fg),
                Val::F(params.time),
                Val::F(params.seed as f32),
            ];
            let mut run_args = Vec::with_capacity(args.len() + extern_args.len());
            run_args.extend_from_slice(&args);
            run_args.extend_from_slice(&extern_args);
            vm.set_seed(mix_seed(params.seed, i as u32, params.time.to_bits()));
            apply_result(
                &mut out,
                    vm.run(shader.entry, &run_args),
                ch,
                orig_fg,
                orig_bg,
                params.mode,
                shader.entry_ret,
            );
                if let Some(err) = vm.last_error.clone() {
                    return Err(err);
                }
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
                let orig_fg = glyph.original_fg;
                let orig_bg = glyph.original_bg;
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
                    Val::C(orig_fg),
                    Val::F(params.time),
                    Val::F(params.seed as f32),
                ];
                let mut run_args = Vec::with_capacity(args.len() + extern_args.len());
                run_args.extend_from_slice(&args);
                run_args.extend_from_slice(&extern_args);
                vm.set_seed(mix_seed(
                    params.seed,
                    global_i as u32,
                    params.time.to_bits(),
                ));
                apply_result(
                    &mut out,
                    vm.run(shader.entry, &run_args),
                    ch,
                    orig_fg,
                    orig_bg,
                    params.mode,
                    shader.entry_ret,
                );
                if let Some(err) = vm.last_error.clone() {
                    return Err(err);
                }
                global_i += 1;
            }
            if row_idx + 1 < rows {
                if !matches!(params.mode, TextMode::Ascii) {
                    out.push_str("\x1b[0m");
                }
                out.push('\n');
            }
        }
    }

    if !matches!(params.mode, TextMode::Ascii) {
        out.push_str("\x1b[0m");
    }
    Ok(out)
}

/// Run shader over text using raw `.ctsl` bytes.
pub fn render_bytes(
    ctsl: &[u8],
    text: &str,
    params: &RenderParams,
) -> Result<String, String> {
    let shader = CompiledShader::from_bytes(ctsl)
        .map_err(|e| e.to_string())?;
    render(&shader, text, params)
}

fn ds_read_u8(code: &[u8], ip: &mut usize) -> Option<u8> {
    let v = *code.get(*ip)?;
    *ip += 1;
    Some(v)
}

fn ds_read_i16(code: &[u8], ip: &mut usize) -> Option<i16> {
    let a = *code.get(*ip)?;
    let b = *code.get(*ip + 1)?;
    *ip += 2;
    Some(i16::from_le_bytes([a, b]))
}

fn ds_read_f32(code: &[u8], ip: &mut usize) -> Option<f32> {
    let a = *code.get(*ip)?;
    let b = *code.get(*ip + 1)?;
    let c = *code.get(*ip + 2)?;
    let d = *code.get(*ip + 3)?;
    *ip += 4;
    Some(f32::from_le_bytes([a, b, c, d]))
}

fn op_name(op: u8) -> &'static str {
    match op {
        o if o == Op::PushF as u8 => "PushF",
        o if o == Op::PushC as u8 => "PushC",
        o if o == Op::Pop as u8 => "Pop",
        o if o == Op::Load as u8 => "Load",
        o if o == Op::Store as u8 => "Store",
        o if o == Op::AddF as u8 => "AddF",
        o if o == Op::SubF as u8 => "SubF",
        o if o == Op::MulF as u8 => "MulF",
        o if o == Op::DivF as u8 => "DivF",
        o if o == Op::ModF as u8 => "ModF",
        o if o == Op::NegF as u8 => "NegF",
        o if o == Op::AbsF as u8 => "AbsF",
        o if o == Op::AddC as u8 => "AddC",
        o if o == Op::SubC as u8 => "SubC",
        o if o == Op::MulC as u8 => "MulC",
        o if o == Op::DivC as u8 => "DivC",
        o if o == Op::MulCF as u8 => "MulCF",
        o if o == Op::EqF as u8 => "EqF",
        o if o == Op::NeF as u8 => "NeF",
        o if o == Op::LtF as u8 => "LtF",
        o if o == Op::GtF as u8 => "GtF",
        o if o == Op::LeF as u8 => "LeF",
        o if o == Op::GeF as u8 => "GeF",
        o if o == Op::AndF as u8 => "AndF",
        o if o == Op::OrF as u8 => "OrF",
        o if o == Op::NotF as u8 => "NotF",
        o if o == Op::Sin as u8 => "Sin",
        o if o == Op::Cos as u8 => "Cos",
        o if o == Op::Tan as u8 => "Tan",
        o if o == Op::Asin as u8 => "Asin",
        o if o == Op::Acos as u8 => "Acos",
        o if o == Op::Atan as u8 => "Atan",
        o if o == Op::Atan2 as u8 => "Atan2",
        o if o == Op::Sqrt as u8 => "Sqrt",
        o if o == Op::Pow as u8 => "Pow",
        o if o == Op::Exp as u8 => "Exp",
        o if o == Op::Log as u8 => "Log",
        o if o == Op::Log2 as u8 => "Log2",
        o if o == Op::Floor as u8 => "Floor",
        o if o == Op::Ceil as u8 => "Ceil",
        o if o == Op::Round as u8 => "Round",
        o if o == Op::Fract as u8 => "Fract",
        o if o == Op::Min2 as u8 => "Min2",
        o if o == Op::Max2 as u8 => "Max2",
        o if o == Op::Clamp as u8 => "Clamp",
        o if o == Op::Mix as u8 => "Mix",
        o if o == Op::Step as u8 => "Step",
        o if o == Op::Smoothstep as u8 => "Smoothstep",
        o if o == Op::Sign as u8 => "Sign",
        o if o == Op::Length2 as u8 => "Length2",
        o if o == Op::Rgb as u8 => "Rgb",
        o if o == Op::Rgba as u8 => "Rgba",
        o if o == Op::Hsl as u8 => "Hsl",
        o if o == Op::Hsv as u8 => "Hsv",
        o if o == Op::Gray as u8 => "Gray",
        o if o == Op::Mix2C as u8 => "Mix2C",
        o if o == Op::GetR as u8 => "GetR",
        o if o == Op::GetG as u8 => "GetG",
        o if o == Op::GetB as u8 => "GetB",
        o if o == Op::IsSpace as u8 => "IsSpace",
        o if o == Op::IsDigit as u8 => "IsDigit",
        o if o == Op::IsAlpha as u8 => "IsAlpha",
        o if o == Op::IsUpper as u8 => "IsUpper",
        o if o == Op::IsLower as u8 => "IsLower",
        o if o == Op::Jmp as u8 => "Jmp",
        o if o == Op::JmpZ as u8 => "JmpZ",
        o if o == Op::Ret as u8 => "Ret",
        o if o == Op::Call as u8 => "Call",
        o if o == Op::Rand as u8 => "Rand",
        o if o == Op::RandBetween as u8 => "RandBetween",
        o if o == Op::MakeGlyphCharFg as u8 => "MakeGlyphCharFg",
        o if o == Op::MakeGlyphFgBg as u8 => "MakeGlyphFgBg",
        o if o == Op::MakeGlyphCharFgBg as u8 => "MakeGlyphCharFgBg",
        o if o == Op::HashF as u8 => "HashF",
        o if o == Op::Vec2 as u8 => "Vec2",
        o if o == Op::Vec3 as u8 => "Vec3",
        o if o == Op::Vec4 as u8 => "Vec4",
        o if o == Op::Mat2 as u8 => "Mat2",
        o if o == Op::Mat3 as u8 => "Mat3",
        o if o == Op::Mat4 as u8 => "Mat4",
        o if o == Op::Swizzle as u8 => "Swizzle",
        o if o == Op::Dot as u8 => "Dot",
        o if o == Op::Cross as u8 => "Cross",
        o if o == Op::Normalize as u8 => "Normalize",
        o if o == Op::Reflect as u8 => "Reflect",
        o if o == Op::Refract as u8 => "Refract",
        o if o == Op::CallExt as u8 => "CallExt",
        o if o == Op::ArrayMake as u8 => "ArrayMake",
        o if o == Op::ArrayGet as u8 => "ArrayGet",
        o if o == Op::ArraySet as u8 => "ArraySet",
        _ => "Unknown",
    }
}

/// Pretty-print bytecode for all functions in a compiled shader.
pub fn disassemble(shader: &CompiledShader) -> String {
    let mut out = String::new();
    out.push_str(&format!("entry_fn: {}\n", shader.entry));
    out.push_str(&format!("entry_ret: {:?}\n", shader.entry_ret));
    out.push_str(&format!("extern_count: {}\n", shader.externs.len()));
    for (i, ext) in shader.externs.iter().enumerate() {
        let default_text = ext
            .default
            .as_ref()
            .map(format_extern_default)
            .unwrap_or_else(|| "<none>".to_string());
        out.push_str(&format!(
            "  extern[{}]: name={} type={:?} default={}\n",
            i,
            ext.name,
            ext.ty,
            default_text
        ));
    }
    out.push_str(&format!("fn_count: {}\n", shader.fns.len()));

    for (fi, f) in shader.fns.iter().enumerate() {
        out.push_str(&format!(
            "\nfn[{}] {} params={} code_len={}\n",
            fi,
            f.name,
            f.params,
            f.code.len()
        ));
        let code = &f.code;
        let mut ip = 0usize;
        while ip < code.len() {
            let at = ip;
            let op = code[ip];
            ip += 1;
            out.push_str(&format!("  {:04} {:<18}", at, op_name(op)));

            let truncated = match op {
                o if o == Op::PushF as u8 => {
                    if let Some(v) = ds_read_f32(code, &mut ip) {
                        out.push_str(&format!("{}", v));
                        false
                    } else {
                        true
                    }
                }
                o if o == Op::PushC as u8 => {
                    if let (Some(r), Some(g), Some(b)) = (
                        ds_read_f32(code, &mut ip),
                        ds_read_f32(code, &mut ip),
                        ds_read_f32(code, &mut ip),
                    ) {
                        out.push_str(&format!("{}, {}, {}", r, g, b));
                        false
                    } else {
                        true
                    }
                }
                o if o == Op::Load as u8 || o == Op::Store as u8 || o == Op::ArrayMake as u8 => {
                    if let Some(v) = ds_read_u8(code, &mut ip) {
                        out.push_str(&format!("{}", v));
                        false
                    } else {
                        true
                    }
                }
                o if o == Op::Jmp as u8 || o == Op::JmpZ as u8 => {
                    if let Some(off) = ds_read_i16(code, &mut ip) {
                        let target = ip as i64 + off as i64;
                        out.push_str(&format!("off={} target={}", off, target));
                        false
                    } else {
                        true
                    }
                }
                o if o == Op::Call as u8 => {
                    if let (Some(idx), Some(argc)) =
                        (ds_read_u8(code, &mut ip), ds_read_u8(code, &mut ip))
                    {
                        out.push_str(&format!("fn={} argc={}", idx, argc));
                        false
                    } else {
                        true
                    }
                }
                o if o == Op::CallExt as u8 => {
                    if let (Some(idx), Some(argc), Some(wcount)) = (
                        ds_read_u8(code, &mut ip),
                        ds_read_u8(code, &mut ip),
                        ds_read_u8(code, &mut ip),
                    ) {
                        out.push_str(&format!("fn={} argc={} writebacks=[", idx, argc));
                        let mut ok = true;
                        for i in 0..wcount {
                            let Some(callee_slot) = ds_read_u8(code, &mut ip) else {
                                ok = false;
                                break;
                            };
                            let Some(caller_slot) = ds_read_u8(code, &mut ip) else {
                                ok = false;
                                break;
                            };
                            if i > 0 {
                                out.push_str(", ");
                            }
                            out.push_str(&format!("({}->{})", callee_slot, caller_slot));
                        }
                        out.push(']');
                        !ok
                    } else {
                        true
                    }
                }
                o if o == Op::Swizzle as u8 => {
                    if let Some(len) = ds_read_u8(code, &mut ip) {
                        if ip + len as usize <= code.len() {
                            let mask = &code[ip..ip + len as usize];
                            ip += len as usize;
                            let text: String = mask.iter().map(|b| *b as char).collect();
                            out.push_str(&format!("\"{}\"", text));
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                }
                _ => false,
            };

            if truncated {
                out.push_str("<truncated>");
                out.push('\n');
                break;
            }
            out.push('\n');
        }
    }

    out
}

/// Pretty-print bytecode from raw `.ctsl` bytes.
pub fn disassemble_bytes(ctsl: &[u8]) -> Result<String, &'static str> {
    let shader = CompiledShader::from_bytes(ctsl)?;
    Ok(disassemble(&shader))
}
