#![cfg(feature = "editor")]
use ansi_parser::{AnsiParser, AnsiSequence, Output};
use eframe::egui;
use egui::{Color32, FontId, Id, Key, TextFormat};
use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;
use tsl::{RenderParams, TextMode, compile, render, utils};

const SHADER_PRESETS: [(&str, &str); 12] = [
    (
        "Rainbow (animated)",
        "fn main(t, i, len, x, y, col_i, row_i) -> color\n  let hue = fract(x * 0.8 + y * 0.35 + time() * 0.18)\n  return hsv(hue, 1.0, 1.0)\nend\n",
    ),
    (
        "Sunset Ramp",
        "fn main(t, i, len, x, y, col_i, row_i) -> color\n  let sky = mixc(rgb(15, 30, 80), rgb(255, 130, 70), y)\n  return mixc(sky, rgb(255, 210, 120), x * 0.4)\nend\n",
    ),
    (
        "Rectangle",
        "fn main(t, i, len, x, y, col_i, row_i) -> color\n  let inside = step(0.08, x) * step(0.08, y) * step(x, 0.92) * step(y, 0.92)\n  if inside == 0.0 do\n    return rgb(20, 35, 80)\n  end\n  let pulse = 0.5 + 0.5 * sin(time() * 2.0 + x * 8.0)\n  return mixc(rgb(255, 180, 50), rgb(255, 240, 170), pulse)\nend\n",
    ),
    (
        "Trans Flag",
        "fn stripe(y) -> color\n  let band = floor(clamp(y, 0.0, 0.9999) * 5.0)\n  if band == 0.0 do return rgb(91, 206, 250) end\n  if band == 1.0 do return rgb(245, 169, 184) end\n  if band == 2.0 do return rgb(255, 255, 255) end\n  if band == 3.0 do return rgb(245, 169, 184) end\n  return rgb(91, 206, 250)\nend\n\nfn main(t, i, len, x, y) -> color\n  let flag = stripe(y)\n  let grain = rand(i + seed() * 0.01) * 0.08\n  return mixc(flag, rgb(255, 255, 255), grain)\nend\n",
    ),
    (
        "Char Colors",
        "fn main(t, i, len, x, y, col_i, row_i, c) -> color\n  if c == '#' do return rgb(255, 120, 255) end\n  if c == 'A' do return rgb(255, 255, 120) end\n  if c == ' ' do return rgb(80, 80, 80) end\n  if c >= '0' && c <= '9' do return rgb(255, 208, 64) end\n  if c >= 'A' && c <= 'Z' do return rgb(120, 200, 255) end\n  if c >= 'a' && c <= 'z' do return rgb(120, 255, 160) end\n  return rgb(255, 120, 160)\nend\n",
    ),
    (
        "Char Replace Matrix",
        "fn main(t, i, len, x, y, col_i, row_i, c) -> (char, color)\n  let seedv = i * 17.0 + floor(time() * 8.0)\n  let r = rand(seedv)\n  let new_c = floor(33.0 + r * 94.0)\n  let glow = 0.3 + 0.7 * rand(seedv + 3.0)\n  return (new_c, rgb(20.0 * glow, 255.0 * glow, 80.0 * glow))\nend\n",
    ),
    (
        "Checker FG/BG",
        "fn main(t, i, len, x, y, col_i, row_i, c) -> (color, color)\n  let cx = floor(col_i * 0.5)\n  let cy = floor(row_i * 0.5)\n  let checker = (cx + cy) % 2.0\n  if checker == 0.0 do\n    return (rgb(240, 240, 240), rgb(30, 60, 120))\n  end\n  return (rgb(40, 30, 20), rgb(240, 180, 110))\nend\n",
    ),
    (
        "Neon Title",
        "fn main(t, i, len, x, y, col_i, row_i, c) -> (char, color, color)\n  let pulse = 0.55 + 0.45 * sin(time() * 3.0 + x * 10.0)\n  let fg = mixc(rgb(120, 220, 255), rgb(255, 100, 220), pulse)\n  let bg = rgb(8, 10, 28)\n  if c == ' ' do\n    return (' ', rgb(160, 160, 160), rgb(8, 10, 28))\n  end\n  return (c, fg, bg)\nend\n",
    ),
    (
        "Wave Distort",
        "fn main(t, i, len, x, y, col_i, row_i, c) -> (char, color)\n  let phase = sin(time() * 2.5 + y * 12.0)\n  let shift = floor((phase + 1.0) * 6.0)\n  let new_c = 33.0 + ((c + shift) % 94.0)\n  let hue = fract(x + time() * 0.1)\n  return (new_c, hsv(hue, 0.9, 1.0))\nend\n",
    ),
    (
        "Loop Bands",
        "fn main(t, i, len, x, y, col_i, row_i, c) -> color\n  let k = 0.0\n  let acc = 0.0\n  while k < 6.0 do\n    let band = smoothstep(k * 0.16, k * 0.16 + 0.1, y)\n    acc = acc + band * (0.6 / (k + 1.0))\n    k = k + 1.0\n  end\n  let v = clamp(acc + 0.2 * sin(time() + x * 7.0), 0.0, 1.0)\n  return hsv(fract(v + x * 0.25), 0.8, v)\nend\n",
    ),
    (
        "Seeded Static",
        "fn main(t, i, len, x, y, col_i, row_i, c) -> (char, color)\n  let n = rand(i + seed() * 0.123)\n  if n < 0.82 do\n    return ('.', rgb(90, 100, 120))\n  end\n  if n < 0.95 do\n    return ('*', rgb(170, 200, 255))\n  end\n  return ('#', rgb(255, 255, 255))\nend\n",
    ),
    (
        "Hex Dump",
        "fn nib(v) -> char\n  if v < 10.0 do return 48.0 + v end\n  return 55.0 + v\nend\n\nfn main(t, i, len, x, y, col_i, row_i, c) -> (char, color)\n  let which = col_i % 3.0\n  if which == 0.0 do\n    return (nib(floor(c / 16.0) % 16.0), rgb(255, 180, 90))\n  end\n  if which == 1.0 do\n    return (nib(c % 16.0), rgb(255, 220, 140))\n  end\n  return (' ', rgb(120, 120, 120))\nend\n",
    ),
];

const TEXT_PRESETS: [(&str, &str); 6] = [
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
    (
        "Quote",
        "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG\nPack my box with five dozen liquor jugs.\nSphinx of black quartz, judge my vow.",
    ),
    (
        "Code",
        "fn shade(x, y) -> color\n  let edge = step(0.1, x) * step(0.1, y)\n  return mixc(rgb(30,40,90), rgb(255,180,80), edge)\nend",
    ),
    (
        "Lorem Ipsum",
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\nUt enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.\nDuis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.\nExcepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.\n\nSed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam.\nEaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo.\nNemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos.\nQui ratione voluptatem sequi nesciunt, neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur.\n\nAdipisci velit, sed quia non numquam eius modi tempora incidunt ut labore et dolore magnam aliquam quaerat voluptatem.\nUt enim ad minima veniam, quis nostrum exercitationem ullam corporis suscipit laboriosam, nisi ut aliquid ex ea commodi.\nConsequatur? Quis autem vel eum iure reprehenderit qui in ea voluptate velit esse quam nihil molestiae consequatur.\nVel illum qui dolorem eum fugiat quo voluptas nulla pariatur?\n\nAt vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium voluptatum deleniti atque corrupti.\nQuos dolores et quas molestias excepturi sint occaecati cupiditate non provident, similique sunt in culpa qui officia deserunt.\nMollitia animi, id est laborum et dolorum fuga. Et harum quidem rerum facilis est et expedita distinctio.\nNam libero tempore, cum soluta nobis est eligendi optio cumque nihil impedit quo minus id quod maxime placeat facere possimus.",
    ),
];

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
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
    output_layout: egui::text::LayoutJob,
    unescaped_input_source: String,
    unescaped_input_cache: String,
    error: Option<String>,
    compiled_shader: Option<tsl::CompiledShader>,
    compiled_shader_source: String,
    shader_error_span: Option<(usize, usize)>,
    shader_uses_time: bool,
    animation_mode: AnimationMode,
    preview_fps: f32,
    preview_last_frame: Option<Instant>,
    shader_preset_idx: usize,
    text_preset_idx: usize,
    split_ratio: f32,
    text_split_ratio: f32,
    show_externs: bool,
    extern_name_col_width: f32,
    extern_rows: Vec<ExternRow>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AnimationMode {
    Auto,
    On,
    Off,
}

impl AnimationMode {
    fn label(self) -> &'static str {
        match self {
            AnimationMode::Auto => "Auto",
            AnimationMode::On => "On",
            AnimationMode::Off => "Off",
        }
    }
}

fn shader_uses_animation_intrinsics(code: &str) -> bool {
    let bytes = code.as_bytes();
    let mut i = 0usize;

    while i < code.len() {
        let ch = bytes[i] as char;

        if ch == '#' {
            i += 1;
            while i < code.len() && (bytes[i] as char) != '\n' {
                i += 1;
            }
            continue;
        }

        if ch == '/' && i + 1 < code.len() && (bytes[i + 1] as char) == '/' {
            i += 2;
            while i < code.len() && (bytes[i] as char) != '\n' {
                i += 1;
            }
            continue;
        }

        if ch == '-' && i + 1 < code.len() && (bytes[i + 1] as char) == '-' {
            i += 2;
            while i < code.len() && (bytes[i] as char) != '\n' {
                i += 1;
            }
            continue;
        }

        if ch == '"' || ch == '\'' {
            let quote = ch;
            i += 1;
            let mut escaped = false;
            while i < code.len() {
                let c = bytes[i] as char;
                i += 1;
                if escaped {
                    escaped = false;
                    continue;
                }
                if c == '\\' {
                    escaped = true;
                    continue;
                }
                if c == quote {
                    break;
                }
            }
            continue;
        }

        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            i += 1;
            while i < code.len() {
                let c = bytes[i] as char;
                if c.is_ascii_alphanumeric() || c == '_' {
                    i += 1;
                } else {
                    break;
                }
            }
            let ident = &code[start..i];
            if ident == "time" || ident == "rand" || ident == "seed" {
                let mut j = i;
                while j < code.len() && (bytes[j] as char).is_ascii_whitespace() {
                    j += 1;
                }
                if j < code.len() && (bytes[j] as char) == '(' {
                    return true;
                }
            }
            continue;
        }

        i += 1;
    }

    false
}

#[derive(Default, Clone)]
struct ExternRow {
    name: String,
    value: String,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            shader_code: SHADER_PRESETS[0].1.to_string(),
            input_text: TEXT_PRESETS[0].1.to_string(),
            output: String::new(),
            output_layout: egui::text::LayoutJob::default(),
            unescaped_input_source: String::new(),
            unescaped_input_cache: String::new(),
            error: None,
            compiled_shader: None,
            compiled_shader_source: String::new(),
            shader_error_span: None,
            shader_uses_time: shader_uses_animation_intrinsics(SHADER_PRESETS[0].1),
            animation_mode: AnimationMode::Auto,
            preview_fps: 0.0,
            preview_last_frame: None,
            shader_preset_idx: 0,
            text_preset_idx: 0,
            split_ratio: 0.36,
            text_split_ratio: 0.5,
            show_externs: false,
            extern_name_col_width: 170.0,
            extern_rows: vec![ExternRow::default()],
        }
    }
}

impl Editor {
    fn should_tick_preview(&self) -> bool {
        if self.input_text.is_empty() || self.error.is_some() {
            return false;
        }

        match self.animation_mode {
            AnimationMode::Auto => self.shader_uses_time,
            AnimationMode::On => true,
            AnimationMode::Off => false,
        }
    }

    fn update_fps(&mut self, running: bool) {
        if !running {
            self.preview_last_frame = None;
            self.preview_fps = 0.0;
            return;
        }

        let now = Instant::now();
        if let Some(prev) = self.preview_last_frame {
            let dt = (now - prev).as_secs_f32();
            if dt > 0.0 {
                let inst_fps = 1.0 / dt;
                self.preview_fps = if self.preview_fps <= 0.0 {
                    inst_fps
                } else {
                    self.preview_fps * 0.9 + inst_fps * 0.1
                };
            }
        }
        self.preview_last_frame = Some(now);
    }

    fn normalize_extern_rows(&mut self) {
        let mut kept: Vec<ExternRow> = self
            .extern_rows
            .iter()
            .filter(|row| !row.name.trim().is_empty() || !row.value.trim().is_empty())
            .cloned()
            .collect();
        kept.push(ExternRow::default());
        self.extern_rows = kept;
    }

    fn collect_externs(&self) -> Result<HashMap<String, tsl::ExternValue>, String> {
        let mut externs = HashMap::new();

        for row in &self.extern_rows {
            let name = row.name.trim();
            let value = row.value.trim();

            if name.is_empty() && value.is_empty() {
                continue;
            }
            if name.is_empty() {
                return Err("extern name cannot be empty when value is set".to_string());
            }
            if value.is_empty() {
                return Err(format!("extern '{}' has no value", name));
            }

            let parsed = parse_extern_value(value)
                .ok_or_else(|| format!("could not parse extern value {} for '{}'", value, name))?;
            externs.insert(name.to_string(), parsed);
        }

        Ok(externs)
    }

    fn rerender(&mut self) {
        if self.input_text.is_empty() {
            self.output = String::new();
            self.error = None;
            return;
        }

        if self.shader_code != self.compiled_shader_source {
            match compile(&self.shader_code) {
                Ok(shader) => {
                    self.compiled_shader = Some(shader);
                    self.compiled_shader_source = self.shader_code.clone();
                    self.shader_error_span = None;
                    self.shader_uses_time = shader_uses_animation_intrinsics(&self.shader_code);
                }
                Err(e) => {
                    self.compiled_shader = None;
                    self.compiled_shader_source = self.shader_code.clone();
                    self.shader_error_span = extract_error_span(&e);
                    self.shader_uses_time = shader_uses_animation_intrinsics(&self.shader_code);
                    self.error = Some(e);
                    return;
                }
            }
        }

        let Some(shader) = self.compiled_shader.as_ref() else {
            return;
        };

        let externs = match self.collect_externs() {
            Ok(e) => e,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };

        let params = RenderParams {
            mode: TextMode::Ansi24,
            time: utils::time(),
            externs,
            ..Default::default()
        };
        if self.unescaped_input_source != self.input_text {
            self.unescaped_input_source = self.input_text.clone();
            self.unescaped_input_cache = unescape_input(&self.input_text);
        }

        match render(shader, &self.unescaped_input_cache, &params) {
            Ok(output) => {
                if self.output != output {
                    self.output = output;
                    self.output_layout = ansi_layout_job(&self.output);
                }
                self.error = None;
            }
            Err(e) => {
                self.error = Some(e);
            }
        }
    }
}

impl eframe::App for Editor {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let viewport_visible = ctx.input(|i| i.viewport().visible().unwrap_or(true));
        if !viewport_visible {
            self.update_fps(false);
            return;
        }

        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Q)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let mut changed = false;
        egui::CentralPanel::default().show(ui, |ui| {
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
                        let span = self.shader_error_span;
                        let mut layouter = move |ui: &egui::Ui,
                                                 text: &dyn egui::TextBuffer,
                                                 wrap_width: f32| {
                            let mut job = shader_layout_job(text.as_str(), span, ui.visuals().dark_mode);
                            job.wrap.max_width = wrap_width;
                            ui.ctx().fonts_mut(|f| f.layout_job(job))
                        };
                        let r = ui.add_sized(
                            avail,
                            egui::TextEdit::multiline(&mut self.shader_code)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace)
                                .layouter(&mut layouter),
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
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("FPS {:.1}", self.preview_fps)).monospace());
                        egui::ComboBox::from_id_salt("animation_mode")
                            .selected_text(format!("Anim {}", self.animation_mode.label()))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.animation_mode, AnimationMode::Auto, "Anim Auto");
                                ui.selectable_value(&mut self.animation_mode, AnimationMode::On, "Anim On");
                                ui.selectable_value(&mut self.animation_mode, AnimationMode::Off, "Anim Off");
                            });
                        if ui.button("Externs").clicked() {
                            self.show_externs = !self.show_externs;
                        }
                        if ui.button("Copy ANSI").clicked() {
                            let escaped = escape_ansi_for_copy(&self.output);
                            ui.ctx().copy_text(escaped);
                        }
                    });
                });
                ui.separator();

                if self.show_externs {
                    self.normalize_extern_rows();
                    ui.label("Externs (name = value)");

                    let divider_w = 8.0;
                    let divider_pad = 4.0;
                    let divider_total_w = divider_w + divider_pad * 2.0;
                    let row_gap = 4.0;
                    let min_name_w = 90.0;
                    let min_value_w = 180.0;
                    let row_h = ui.spacing().interact_size.y;
                    let table_w = ui.available_width().max(min_name_w + min_value_w + divider_total_w);
                    let max_name_w = (table_w - min_value_w - divider_total_w).max(min_name_w);
                    self.extern_name_col_width = self.extern_name_col_width.clamp(min_name_w, max_name_w);
                    let row_count = self.extern_rows.len() + 1;
                    let table_h = row_count as f32 * row_h + (row_count.saturating_sub(1)) as f32 * row_gap;
                    let table_top = ui.cursor().min.y;
                    let table_left = ui.cursor().min.x;

                    let mut sep_x = table_left + self.extern_name_col_width + divider_pad + divider_w * 0.5;
                    let sep_rect = egui::Rect::from_min_max(
                        egui::pos2(sep_x - divider_w * 0.5, table_top),
                        egui::pos2(sep_x + divider_w * 0.5, table_top + table_h),
                    );
                    let sep_resp = ui.interact(
                        sep_rect,
                        Id::new("extern_table_splitter"),
                        egui::Sense::click_and_drag(),
                    );

                    if sep_resp.hovered() || sep_resp.dragged() {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
                    }
                    if sep_resp.dragged() {
                        let dx = ui.input(|i| i.pointer.delta().x);
                        self.extern_name_col_width =
                            (self.extern_name_col_width + dx).clamp(min_name_w, max_name_w);
                        sep_x = table_left + self.extern_name_col_width + divider_pad + divider_w * 0.5;
                    }

                    let value_col_width = table_w - self.extern_name_col_width - divider_total_w;
                    let mut externs_changed = false;
                    ui.scope(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.spacing_mut().item_spacing.y = row_gap;

                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [self.extern_name_col_width, row_h],
                                egui::Label::new(egui::RichText::new("Name").strong()),
                            );
                            ui.add_space(divider_pad);
                            let _ = ui.allocate_exact_size(egui::vec2(divider_w, row_h), egui::Sense::hover());
                            ui.add_space(divider_pad);
                            ui.add_sized(
                                [value_col_width, row_h],
                                egui::Label::new(egui::RichText::new("Value").strong()),
                            );
                        });

                        for row in &mut self.extern_rows {
                            ui.horizontal(|ui| {
                                let name_resp = ui.add_sized(
                                    [self.extern_name_col_width, row_h],
                                    egui::TextEdit::singleline(&mut row.name).hint_text("accent"),
                                );
                                ui.add_space(divider_pad);
                                let _ = ui.allocate_exact_size(egui::vec2(divider_w, row_h), egui::Sense::hover());
                                ui.add_space(divider_pad);
                                let value_resp = ui.add_sized(
                                    [value_col_width, row_h],
                                    egui::TextEdit::singleline(&mut row.value)
                                        .hint_text("#F5A9B8"),
                                );
                                if name_resp.changed() || value_resp.changed() {
                                    externs_changed = true;
                                }
                            });
                        }
                    });

                    let sep_stroke = ui.visuals().widgets.noninteractive.bg_stroke;
                    let x = sep_x;
                    ui.painter().line_segment(
                        [egui::pos2(x, table_top), egui::pos2(x, table_top + table_h)],
                        sep_stroke,
                    );

                    if sep_resp.dragged() {
                        ctx.request_repaint();
                    }
                    if externs_changed {
                        self.normalize_extern_rows();
                        changed = true;
                    }
                    ui.separator();
                }

                egui::ScrollArea::both().show(ui, |ui| {
                    ui.label(self.output_layout.clone());
                });
            });
        });

        if changed {
            self.rerender();
        }

        let ticking = self.should_tick_preview();
        self.update_fps(ticking);
        if ticking {
            self.rerender();
            ctx.request_repaint();
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
}

fn extract_error_span(msg: &str) -> Option<(usize, usize)> {
    for line in msg.lines() {
        let Some(byte_idx) = line.find("byte ") else {
            continue;
        };
        let rest = &line[byte_idx + 5..];
        let mut start_digits = String::new();
        let mut end_digits = String::new();
        let mut seen_dots = false;
        for ch in rest.chars() {
            if !seen_dots {
                if ch.is_ascii_digit() {
                    start_digits.push(ch);
                    continue;
                }
                if ch == '.' {
                    seen_dots = true;
                    continue;
                }
                if !start_digits.is_empty() {
                    break;
                }
            } else {
                if ch.is_ascii_digit() {
                    end_digits.push(ch);
                    continue;
                }
                if !end_digits.is_empty() {
                    break;
                }
            }
        }
        if let (Ok(start), Ok(end)) = (start_digits.parse::<usize>(), end_digits.parse::<usize>()) {
            return Some((start, end.max(start)));
        }
    }
    None
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn next_non_ws_char(text: &str, mut idx: usize) -> Option<char> {
    while idx < text.len() {
        let ch = text[idx..].chars().next()?;
        if !ch.is_whitespace() {
            return Some(ch);
        }
        idx += ch.len_utf8();
    }
    None
}

fn token_color(word: &str, dark_mode: bool) -> Option<Color32> {
    let kw = if dark_mode {
        Color32::from_rgb(236, 142, 84)
    } else {
        Color32::from_rgb(150, 72, 22)
    };
    let ty = if dark_mode {
        Color32::from_rgb(128, 198, 255)
    } else {
        Color32::from_rgb(33, 90, 145)
    };

    match word {
        "fn" | "let" | "if" | "else" | "while" | "do" | "end" | "return" | "and" | "or" | "extern"
        | "not" => Some(kw),
        "true" | "false" | "color" | "char" | "bool" | "number" | "float" => Some(ty),
        _ => None,
    }
}

fn paren_color(level: usize, dark_mode: bool) -> Color32 {
    const DARK: [Color32; 6] = [
        Color32::from_rgb(255, 194, 102),
        Color32::from_rgb(121, 204, 255),
        Color32::from_rgb(158, 230, 147),
        Color32::from_rgb(248, 155, 218),
        Color32::from_rgb(255, 228, 122),
        Color32::from_rgb(171, 187, 255),
    ];
    const LIGHT: [Color32; 6] = [
        Color32::from_rgb(176, 83, 0),
        Color32::from_rgb(0, 104, 169),
        Color32::from_rgb(26, 125, 47),
        Color32::from_rgb(141, 37, 125),
        Color32::from_rgb(149, 114, 0),
        Color32::from_rgb(63, 72, 178),
    ];

    let palette = if dark_mode { &DARK } else { &LIGHT };
    palette[level % palette.len()]
}

fn overlaps_span(start: usize, end: usize, span: Option<(usize, usize)>) -> bool {
    let Some((raw_start, raw_end)) = span else {
        return false;
    };

    let s = raw_start.min(raw_end);
    let e = raw_start.max(raw_end);
    start < e && end > s
}

fn append_segment(
    job: &mut egui::text::LayoutJob,
    text: &str,
    start: usize,
    end: usize,
    color: Color32,
    span: Option<(usize, usize)>,
) {
    if end <= start {
        return;
    }
    let mut fmt = TextFormat {
        font_id: FontId::monospace(14.0),
        color,
        ..Default::default()
    };
    if overlaps_span(start, end, span) {
        fmt.underline = egui::Stroke::new(1.0, Color32::from_rgb(255, 110, 110));
    }
    job.append(&text[start..end], 0.0, fmt);
}

fn shader_layout_job(text: &str, span: Option<(usize, usize)>, dark_mode: bool) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let base = if dark_mode {
        Color32::LIGHT_GRAY
    } else {
        Color32::from_rgb(50, 50, 50)
    };
    let number = if dark_mode {
        Color32::from_rgb(181, 206, 168)
    } else {
        Color32::from_rgb(34, 114, 47)
    };
    let string = if dark_mode {
        Color32::from_rgb(214, 157, 133)
    } else {
        Color32::from_rgb(161, 73, 0)
    };
    let comment = if dark_mode {
        Color32::from_rgb(106, 153, 85)
    } else {
        Color32::from_rgb(76, 133, 54)
    };
    let fn_decl = if dark_mode {
        Color32::from_rgb(120, 215, 196)
    } else {
        Color32::from_rgb(0, 122, 102)
    };
    let fn_call = if dark_mode {
        Color32::from_rgb(220, 220, 170)
    } else {
        Color32::from_rgb(128, 95, 0)
    };

    let mut i = 0usize;
    let mut paren_level = 0usize;
    let bytes = text.as_bytes();
    let mut expect_fn_name = false;

    while i < text.len() {
        let ch = text[i..].chars().next().unwrap();
        let ch_len = ch.len_utf8();

        if ch == '/' && bytes.get(i + 1) == Some(&b'/') {
            let start = i;
            i += 2;
            while i < text.len() {
                let c = text[i..].chars().next().unwrap();
                let l = c.len_utf8();
                if c == '\n' {
                    break;
                }
                i += l;
            }
            append_segment(&mut job, text, start, i, comment, span);
            continue;
        }

        if ch == '#' {
            let start = i;
            i += 1;
            while i < text.len() {
                let c = text[i..].chars().next().unwrap();
                let l = c.len_utf8();
                if c == '\n' {
                    break;
                }
                i += l;
            }
            append_segment(&mut job, text, start, i, comment, span);
            continue;
        }

        if ch == '-' && bytes.get(i + 1) == Some(&b'-') {
            let start = i;
            i += 2;
            while i < text.len() {
                let c = text[i..].chars().next().unwrap();
                let l = c.len_utf8();
                if c == '\n' {
                    break;
                }
                i += l;
            }
            append_segment(&mut job, text, start, i, comment, span);
            continue;
        }

        if ch == '"' || ch == '\'' {
            let quote = ch;
            let start = i;
            i += ch_len;
            let mut escaped = false;
            while i < text.len() {
                let c = text[i..].chars().next().unwrap();
                let l = c.len_utf8();
                i += l;

                if escaped {
                    escaped = false;
                    continue;
                }
                if c == '\\' {
                    escaped = true;
                    continue;
                }
                if c == quote {
                    break;
                }
            }
            append_segment(&mut job, text, start, i, string, span);
            continue;
        }

        if ch.is_ascii_digit()
            || (ch == '.'
                && bytes
                    .get(i + 1)
                    .is_some_and(|b| (*b as char).is_ascii_digit()))
        {
            let start = i;
            i += ch_len;
            while i < text.len() {
                let c = text[i..].chars().next().unwrap();
                let l = c.len_utf8();
                if c.is_ascii_digit() || c == '.' || c == '_' {
                    i += l;
                } else {
                    break;
                }
            }
            append_segment(&mut job, text, start, i, number, span);
            continue;
        }

        if is_ident_start(ch) {
            let start = i;
            i += ch_len;
            while i < text.len() {
                let c = text[i..].chars().next().unwrap();
                if is_ident_continue(c) {
                    i += c.len_utf8();
                } else {
                    break;
                }
            }
            let word = &text[start..i];
            let color = if expect_fn_name {
                expect_fn_name = false;
                fn_decl
            } else if word == "fn" {
                expect_fn_name = true;
                token_color(word, dark_mode).unwrap_or(base)
            } else if next_non_ws_char(text, i) == Some('(') {
                fn_call
            } else {
                token_color(word, dark_mode).unwrap_or(base)
            };
            append_segment(&mut job, text, start, i, color, span);
            continue;
        }

        match ch {
            '(' | '[' | '{' => {
                let c = paren_color(paren_level, dark_mode);
                append_segment(&mut job, text, i, i + ch_len, c, span);
                paren_level = paren_level.saturating_add(1);
            }
            ')' | ']' | '}' => {
                paren_level = paren_level.saturating_sub(1);
                let c = paren_color(paren_level, dark_mode);
                append_segment(&mut job, text, i, i + ch_len, c, span);
            }
            _ => append_segment(&mut job, text, i, i + ch_len, base, span),
        }
        i += ch_len;
    }

    job
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
            [a, b] => {
                if let (Some(ch), Some(color_val)) = (parse_char_value(a), parse_extern_value(b))
                    && let tsl::ExternValue::Color(c) = color_val
                {
                    return Some((ch, c).into());
                }

                let a_val = parse_extern_value(a)?;
                let b_val = parse_extern_value(b)?;
                return match (a_val, b_val) {
                    (tsl::ExternValue::Color(fg), tsl::ExternValue::Color(bg)) => Some((fg, bg).into()),
                    _ => None,
                };
            }
            [a, fg, bg] => {
                let ch = parse_char_value(a)?;
                let fg_val = parse_extern_value(fg)?;
                let bg_val = parse_extern_value(bg)?;
                return match (fg_val, bg_val) {
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
    if let Ok(n) = value.parse::<f32>() {
        return Some(n.into());
    }
    if let Some(ch) = parse_char_value(value) {
        return Some(ch.into());
    }
    None
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

fn ansi_layout_job(text: &str) -> egui::text::LayoutJob {
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
    job
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
