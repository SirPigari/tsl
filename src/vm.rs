use crate::compiler::{CompiledShader, Op, ReturnType};

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
    G(GlyphVal),
}

impl Val {
    fn f(self) -> f32 {
        match self {
            Val::F(v) => v,
            Val::C(c) => (c.r + c.g + c.b) / 3.0,
            Val::G(g) => (g.fg.r + g.fg.g + g.fg.b) / 3.0,
        }
    }

    fn c(self) -> Color {
        match self {
            Val::C(c) => c,
            Val::F(v) => Color { r: v, g: v, b: v },
            Val::G(g) => g.fg,
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
    rng_state: u32,
    last_error: Option<String>,
}

impl<'a> Vm<'a> {
    pub fn new(shader: &'a CompiledShader, seed: u32) -> Self {
        Self {
            shader,
            stack: Vec::with_capacity(MAX_STACK),
            frames: Vec::with_capacity(MAX_DEPTH),
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

        let mut frame = Frame::new(fn_idx);
        for (i, &a) in args.iter().enumerate().take(MAX_LOCALS) {
            frame.locals[i] = a;
        }

        loop {
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
                    if !self.push(Val::F(a.f() + b.f())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::SubF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f() - b.f())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::MulF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f() * b.f())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::DivF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    match (a, b) {
                        (Val::C(ca), Val::C(cb)) => {
                            let r = if cb.r == 0.0 { 0.0 } else { ca.r / cb.r };
                            let g = if cb.g == 0.0 { 0.0 } else { ca.g / cb.g };
                            let b = if cb.b == 0.0 { 0.0 } else { ca.b / cb.b };
                            if !self.push(Val::C(Color { r, g, b })) {
                                return Val::F(0.0);
                            }
                        }
                        _ => {
                            let bf = b.f();
                            let af = a.f();
                            if !self.push(Val::F(if bf == 0.0 { 0.0 } else { af / bf })) {
                                return Val::F(0.0);
                            }
                        }
                    }
                }
                o if o == Op::ModF as u8 => {
                    let Some(b) = self.pop() else {
                        return Val::F(0.0);
                    };
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f() % b.f())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::NegF as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(-a.f())) {
                        return Val::F(0.0);
                    }
                }
                o if o == Op::AbsF as u8 => {
                    let Some(a) = self.pop() else {
                        return Val::F(0.0);
                    };
                    if !self.push(Val::F(a.f().abs())) {
                        return Val::F(0.0);
                    }
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
                    if !self.push(Val::F((a.f() * a.f() + b.f() * b.f()).sqrt())) {
                        return Val::F(0.0);
                    }
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
                    if let Some(parent) = self.frames.pop() {
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

pub struct RenderParams {
    pub mode: TextMode,
    pub charset: CharSet,
    pub cols: u32,
    pub time: f32,
    pub seed: u32,
}

impl Default for RenderParams {
    fn default() -> Self {
        Self {
            mode: TextMode::Ansi24,
            charset: CharSet::Ascii,
            cols: 0,
            time: 0.0,
            seed: 0x9e3779b9,
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
                out.push('m');
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
                out.push('m');
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
pub fn render(shader: &CompiledShader, text: &str, params: &RenderParams) -> String {
    if text.is_empty() {
        return String::new();
    }
    if shader.fns.is_empty() || shader.entry >= shader.fns.len() {
        return text.to_string();
    }

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
            vm.set_seed(mix_seed(params.seed, i as u32, params.time.to_bits()));
            apply_result(
                &mut out,
                vm.run(shader.entry, &args),
                ch,
                orig_fg,
                orig_bg,
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
                vm.set_seed(mix_seed(
                    params.seed,
                    global_i as u32,
                    params.time.to_bits(),
                ));
                apply_result(
                    &mut out,
                    vm.run(shader.entry, &args),
                    ch,
                    orig_fg,
                    orig_bg,
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
