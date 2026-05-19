#![cfg(feature = "editor")]
use ansi_parser::{AnsiParser, AnsiSequence, Output};
use eframe::egui;
use egui::{Color32, FontId, Id, Key, TextFormat};
use tsl::{RenderParams, TextMode, compile, render};

const SHADER_PRESETS: [(&str, &str); 6] = [
    (
        "Rainbow (animated)",
        "fn main(t, i, len, x, y, col_i, row_i) -> color\n  return hsv(fract(x + y * 0.2 + time() * 0.2), 1.0, 1.0)\nend\n",
    ),
    (
        "Ramp",
        "fn main(t, i, len, x, y, col_i, row_i) -> color\n  return mixc(rgb(30, 120, 255), rgb(255, 120, 30), x)\nend\n",
    ),
    (
        "Rectangle",
        "fn main(t, i, len, x, y, col_i, row_i) -> color\n  let inside = step(0.05, x) * step(0.05, y) * step(x, 0.95) * step(y, 0.95)\n  if inside == 0.0 do\n    return rgb(35, 35, 90)\n  end\n  return rgb(255, 200, 90)\nend\n",
    ),
    (
        "Trans Flag",
        "fn stripe(y) -> color\n  let band = floor(clamp(y, 0.0, 0.9999) * 5.0)\n\n  if band == 0.0 do return rgb(91, 206, 250) end\n  if band == 1.0 do return rgb(245, 169, 184) end\n  if band == 2.0 do return rgb(255, 255, 255) end\n  if band == 3.0 do return rgb(245, 169, 184) end\n  return rgb(91, 206, 250)\nend\n\nfn main(t, i, len, x, y) -> color\n  let flag = stripe(y)\n\n  let orig = original()\n  let luma = orig.r * 0.299 + orig.g * 0.587 + orig.b * 0.114\n\n  return mixc(flag, rgb(255, 255, 255), max(luma, 0.4) * 0.35)\nend\n",
    ),
    (
        "Char Colors",
        "fn main(t, i, len, x, y, col_i, row_i, c) -> color\n  if c == '#' do return rgb(255, 120, 255) end\n  if c == 'A' do return rgb(255, 255, 120) end\n  if c == ' ' do return rgb(80, 80, 80) end\n  if c >= '0' && c <= '9' do return rgb(255, 208, 64) end\n  if c >= 'A' && c <= 'Z' do return rgb(120, 200, 255) end\n  if c >= 'a' && c <= 'z' do return rgb(120, 255, 160) end\n  return rgb(255, 120, 160)\nend\n",
    ),
    (
        "Char Replace",
        "fn main(t, i, len, x, y, col_i, row_i, c) -> (color, char)\n  let hue = fract(x + time() * 0.15)\n  let new_c = floor(rand(33.0, 127.0))\n  return (hsl(hue, 1.0, 0.55), new_c)\nend\n",
    ),
];

const TEXT_PRESETS: [(&str, &str); 3] = [
    (
        "Rectangle",
        "##################################\n##################################\n##################################\n##################################\n##################################\n##################################\n##################################\n##################################\n##################################\n##################################\n##################################\n##################################\n##################################\n##################################",
    ),
    (
        "Ramp",
        "RAMP PREVIEW\n0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ\n#()+-><.:!abcdefghijklmnopqrstuvwxyz",
    ),
    (
        "Blocks",
        "##########..........##########\n##########..........##########\n##########..........##########",
    ),
];

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 1000.0])
            .with_min_inner_size([800.0, 700.0])
            .with_maximized(true),
        ..Default::default()
    };

    eframe::run_native(
        "TSL Editor",
        options,
        Box::new(|cc| {
            if let Some(ppp) = cc.egui_ctx.native_pixels_per_point() {
                cc.egui_ctx.set_pixels_per_point(ppp);
            }
            Ok(Box::new(Editor::default()))
        }),
    )
}

struct Editor {
    shader_code: String,
    input_text: String,
    output: String,
    error: Option<String>,
    shader_preset_idx: usize,
    text_preset_idx: usize,
    split_ratio: f32,
    text_split_ratio: f32,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            shader_code: SHADER_PRESETS[0].1.to_string(),
            input_text: TEXT_PRESETS[0].1.to_string(),
            output: String::new(),
            error: None,
            shader_preset_idx: 0,
            text_preset_idx: 0,
            split_ratio: 0.36,
            text_split_ratio: 0.5,
        }
    }
}

impl Editor {
    fn rerender(&mut self, time: f32) {
        if self.input_text.is_empty() {
            self.output = String::new();
            self.error = None;
            return;
        }
        let shader = match compile(&self.shader_code) {
            Ok(s) => s,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        let params = RenderParams {
            mode: TextMode::Ansi24,
            time,
            ..Default::default()
        };
        self.output = render(&shader, &unescape_input(&self.input_text), &params);
        self.error = None;
    }
}

impl eframe::App for Editor {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Q)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let mut changed = false;
        let time = ctx.input(|i| i.time as f32);
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.style_mut().override_font_id = Some(FontId::monospace(14.0));
            let panel_rect = ui.max_rect();
            let total_w = panel_rect.width().max(700.0);
            let total_h = panel_rect.height();
            let separator_w = 8.0;
            let min_left_w = 240.0;
            let min_right_w = 320.0;
            let max_left_w = (total_w - min_right_w - separator_w).max(min_left_w);
            let mut left_w = (total_w * self.split_ratio).clamp(min_left_w, max_left_w);
            let splitter_rect = egui::Rect::from_min_size(
                egui::pos2(panel_rect.left() + left_w, panel_rect.top()),
                egui::vec2(separator_w, total_h),
            );
            let splitter_resp = ui.interact(
                splitter_rect,
                Id::new("main_splitter"),
                egui::Sense::click_and_drag(),
            );
            if splitter_resp.dragged() {
                let dx = ui.input(|i| i.pointer.delta().x);
                left_w = (left_w + dx).clamp(min_left_w, max_left_w);
                self.split_ratio = (left_w / total_w).clamp(0.2, 0.8);
                ctx.request_repaint();
            }

            let left_rect = egui::Rect::from_min_max(
                panel_rect.min,
                egui::pos2(splitter_rect.left(), panel_rect.bottom()),
            );
            let right_rect = egui::Rect::from_min_max(
                egui::pos2(splitter_rect.right(), panel_rect.top()),
                panel_rect.max,
            );

            let sep_stroke = ui.visuals().widgets.noninteractive.bg_stroke;
            let x = splitter_rect.center().x;
            ui.painter().line_segment(
                [
                    egui::pos2(x, splitter_rect.top()),
                    egui::pos2(x, splitter_rect.bottom()),
                ],
                sep_stroke,
            );
            if splitter_resp.hovered() || splitter_resp.dragged() {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
            }

            let h_sep_h = 18.0;
            let min_top_h = 80.0;
            let min_bottom_h = 80.0;
            let usable_h = (left_rect.height() - h_sep_h).max(min_top_h + min_bottom_h);
            let max_top_h = usable_h - min_bottom_h;
            let min_top_h = min_top_h.min(max_top_h);
            let top_h = (usable_h * self.text_split_ratio).clamp(min_top_h, max_top_h);
            let hsplit_rect = egui::Rect::from_min_size(
                egui::pos2(left_rect.left(), left_rect.top() + top_h),
                egui::vec2(left_rect.width(), h_sep_h),
            );
            let hsplit_resp = ui.interact(
                hsplit_rect,
                Id::new("text_splitter"),
                egui::Sense::click_and_drag(),
            );
            if hsplit_resp.dragged() {
                let dy = ui.input(|i| i.pointer.delta().y);
                let top_h_now = (usable_h * self.text_split_ratio).clamp(min_top_h, max_top_h);
                let new_top_h = (top_h_now + dy).clamp(min_top_h, max_top_h);
                self.text_split_ratio = (new_top_h / usable_h).clamp(0.0, 1.0);
                ctx.request_repaint();
            }
            let y = hsplit_rect.center().y;
            ui.painter().line_segment(
                [
                    egui::pos2(hsplit_rect.left(), y),
                    egui::pos2(hsplit_rect.right(), y),
                ],
                sep_stroke,
            );
            if hsplit_resp.hovered() || hsplit_resp.dragged() {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeVertical);
            }

            let top_rect = egui::Rect::from_min_max(
                left_rect.min,
                egui::pos2(left_rect.right(), hsplit_rect.top()),
            );
            let bottom_rect = egui::Rect::from_min_max(
                egui::pos2(left_rect.left(), hsplit_rect.bottom()),
                left_rect.max,
            );

            ui.scope_builder(egui::UiBuilder::new().max_rect(top_rect), |ui| {
                ui.heading("Shader");
                ui.separator();

                let prev_shader = self.shader_preset_idx;
                egui::ComboBox::from_label("Shader preset")
                    .selected_text(SHADER_PRESETS[self.shader_preset_idx].0)
                    .show_ui(ui, |ui| {
                        for (idx, (name, _)) in SHADER_PRESETS.iter().enumerate() {
                            ui.selectable_value(&mut self.shader_preset_idx, idx, *name);
                        }
                    });
                if self.shader_preset_idx != prev_shader {
                    self.shader_code = SHADER_PRESETS[self.shader_preset_idx].1.to_string();
                    changed = true;
                }

                ui.add_space(8.0);
                egui::ScrollArea::both()
                    .id_salt("shader_scroll")
                    .show(ui, |ui| {
                        let avail = ui.available_size();
                        let r = ui.add_sized(
                            avail,
                            egui::TextEdit::multiline(&mut self.shader_code)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                        if r.changed() {
                            changed = true;
                        }
                    });
            });

            ui.scope_builder(egui::UiBuilder::new().max_rect(bottom_rect), |ui| {
                ui.heading("Input");

                let prev_text = self.text_preset_idx;
                egui::ComboBox::from_label("Text preset")
                    .selected_text(TEXT_PRESETS[self.text_preset_idx].0)
                    .show_ui(ui, |ui| {
                        for (idx, (name, _)) in TEXT_PRESETS.iter().enumerate() {
                            ui.selectable_value(&mut self.text_preset_idx, idx, *name);
                        }
                    });
                if self.text_preset_idx != prev_text {
                    self.input_text = TEXT_PRESETS[self.text_preset_idx].1.to_string();
                    changed = true;
                }

                ui.add_space(8.0);
                let err_h = if self.error.is_some() { 32.0 } else { 0.0 };
                egui::ScrollArea::both()
                    .id_salt("input_scroll")
                    .max_height(ui.available_height() - err_h)
                    .show(ui, |ui| {
                        let avail = ui.available_size();
                        let r = ui.add_sized(
                            avail.max(egui::vec2(0.0, 40.0)),
                            egui::TextEdit::multiline(&mut self.input_text)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                        if r.changed() {
                            changed = true;
                        }
                    });

                if let Some(err) = &self.error {
                    ui.colored_label(Color32::from_rgb(255, 80, 80), err);
                }
            });

            ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect), |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Preview");
                    if ui.button("Copy ANSI").clicked() {
                        let escaped = escape_ansi_for_copy(&self.output);
                        ui.ctx().copy_text(escaped);
                    }
                });
                ui.separator();
                egui::ScrollArea::both().show(ui, |ui| {
                    render_ansi(ui, &self.output);
                });
            });
        });

        if changed || !self.input_text.is_empty() {
            self.rerender(time);
            ctx.request_repaint();
        }
    }
}

fn unescape_input(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'),
            Some('e') => out.push('\x1b'),
            Some('x') | Some('X') => {
                let mut hex = String::new();
                for _ in 0..2 {
                    if chars.peek().map(|c| c.is_ascii_hexdigit()).unwrap_or(false) {
                        hex.push(chars.next().unwrap());
                    }
                }
                if let Ok(n) = u8::from_str_radix(&hex, 16) {
                    out.push(n as char);
                } else {
                    out.push('\\');
                    out.push('x');
                    out.push_str(&hex);
                }
            }
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn _render_ansi(ui: &mut egui::Ui, text: &str) {
    for item in text.ansi_parse() {
        match &item {
            Output::TextBlock(t) => {
                ui.label(format!("TEXT: {t}"));
            }
            Output::Escape(seq) => {
                ui.label(format!("ESC: {seq:?}"));
            }
        }
    }
}

fn escape_ansi_for_copy(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        match ch {
            '\x1b' => out.push_str("\\x1b"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\\' => out.push_str("\\\\"),
            _ => out.push(ch),
        }
    }
    out
}

fn render_ansi(ui: &mut egui::Ui, text: &str) {
    let mut job = egui::text::LayoutJob::default();
    let mut fg = Color32::LIGHT_GRAY;
    let mut bg = Color32::TRANSPARENT;

    for item in text.ansi_parse() {
        match item {
            Output::TextBlock(t) => {
                job.append(
                    &t,
                    0.0,
                    TextFormat {
                        font_id: FontId::monospace(14.0),
                        color: fg,
                        background: bg,
                        ..Default::default()
                    },
                );
            }
            Output::Escape(AnsiSequence::SetGraphicsMode(modes)) => {
                let m: Vec<u8> = modes.to_vec();
                let mut i = 0;
                while i < m.len() {
                    match m[i] {
                        0 => {
                            fg = Color32::LIGHT_GRAY;
                            bg = Color32::TRANSPARENT;
                        }
                        30..=37 => fg = ansi_color_16(m[i] - 30, false),
                        40..=47 => bg = ansi_color_16(m[i] - 40, false),
                        90..=97 => fg = ansi_color_16(m[i] - 90, true),
                        100..=107 => bg = ansi_color_16(m[i] - 100, true),
                        38 if m.get(i + 1) == Some(&5) => {
                            if let Some(&n) = m.get(i + 2) {
                                fg = xterm256(n);
                            }
                            i += 2;
                        }
                        38 if m.get(i + 1) == Some(&2) => {
                            if let (Some(&r), Some(&g2), Some(&b)) =
                                (m.get(i + 2), m.get(i + 3), m.get(i + 4))
                            {
                                fg = Color32::from_rgb(r, g2, b);
                            }
                            i += 4;
                        }
                        48 if m.get(i + 1) == Some(&5) => {
                            if let Some(&n) = m.get(i + 2) {
                                bg = xterm256(n);
                            }
                            i += 2;
                        }
                        48 if m.get(i + 1) == Some(&2) => {
                            if let (Some(&r), Some(&g2), Some(&b)) =
                                (m.get(i + 2), m.get(i + 3), m.get(i + 4))
                            {
                                bg = Color32::from_rgb(r, g2, b);
                            }
                            i += 4;
                        }
                        _ => {}
                    }
                    i += 1;
                }
            }
            _ => {}
        }
    }
    ui.label(job);
}

fn xterm256(n: u8) -> Color32 {
    match n {
        0 => Color32::from_rgb(0, 0, 0),
        1 => Color32::from_rgb(128, 0, 0),
        2 => Color32::from_rgb(0, 128, 0),
        3 => Color32::from_rgb(128, 128, 0),
        4 => Color32::from_rgb(0, 0, 128),
        5 => Color32::from_rgb(128, 0, 128),
        6 => Color32::from_rgb(0, 128, 128),
        7 => Color32::from_rgb(192, 192, 192),
        8 => Color32::from_rgb(128, 128, 128),
        9 => Color32::from_rgb(255, 0, 0),
        10 => Color32::from_rgb(0, 255, 0),
        11 => Color32::from_rgb(255, 255, 0),
        12 => Color32::from_rgb(0, 0, 255),
        13 => Color32::from_rgb(255, 0, 255),
        14 => Color32::from_rgb(0, 255, 255),
        15 => Color32::from_rgb(255, 255, 255),
        16..=231 => {
            let i = n - 16;
            let r = (i / 36) % 6;
            let g = (i / 6) % 6;
            let b = i % 6;
            let c = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            Color32::from_rgb(c(r), c(g), c(b))
        }
        232..=255 => {
            let v = 8 + (n - 232) * 10;
            Color32::from_rgb(v, v, v)
        }
    }
}

fn ansi_color_16(n: u8, bright: bool) -> Color32 {
    match (n, bright) {
        (0, false) => Color32::from_rgb(0, 0, 0),
        (1, false) => Color32::from_rgb(205, 49, 49),
        (2, false) => Color32::from_rgb(13, 188, 121),
        (3, false) => Color32::from_rgb(229, 229, 16),
        (4, false) => Color32::from_rgb(36, 114, 200),
        (5, false) => Color32::from_rgb(188, 63, 188),
        (6, false) => Color32::from_rgb(17, 168, 205),
        (7, false) => Color32::from_rgb(229, 229, 229),
        (0, true) => Color32::from_rgb(102, 102, 102),
        (1, true) => Color32::from_rgb(241, 76, 76),
        (2, true) => Color32::from_rgb(35, 209, 139),
        (3, true) => Color32::from_rgb(245, 245, 67),
        (4, true) => Color32::from_rgb(59, 142, 234),
        (5, true) => Color32::from_rgb(214, 112, 214),
        (6, true) => Color32::from_rgb(41, 184, 219),
        (7, true) => Color32::from_rgb(255, 255, 255),
        _ => Color32::LIGHT_GRAY,
    }
}
