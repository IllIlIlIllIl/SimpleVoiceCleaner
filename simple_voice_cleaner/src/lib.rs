use nih_plug::editor::Editor;
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use std::num::NonZeroU32;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

const MIN_DB: f32 = -120.0;

#[derive(Clone, Copy)]
struct SavedSettings {
    denoise_enabled: bool,
    denoise_reduction_db: f32,
    denoise_floor_db: f32,
    denoise_softness_db: f32,
    hpf_enabled: bool,
    target_db: f32,
    ride_amount: f32,
    speed_ms: f32,
    noise_floor_db: f32,
    max_boost_db: f32,
    max_cut_db: f32,
    output_gain_db: f32,
    limiter: bool,
}

impl Default for SavedSettings {
    fn default() -> Self {
        Self {
            denoise_enabled: true,
            denoise_reduction_db: 100.0,
            denoise_floor_db: -55.0,
            denoise_softness_db: 12.0,
            hpf_enabled: true,
            target_db: -18.0,
            ride_amount: 70.0,
            speed_ms: 500.0,
            noise_floor_db: -50.0,
            max_boost_db: 6.0,
            max_cut_db: 9.0,
            output_gain_db: 0.0,
            limiter: true,
        }
    }
}

struct SimpleVoiceCleaner {
    params: Arc<SimpleVoiceCleanerParams>,
    sample_rate: f32,
    rms_state: f32,
    denoise_level_state: f32,
    rider_gain_db: f32,
    denoise_gain_db: f32,
    hpf_x1: [f32; 2],
    hpf_y1: [f32; 2],
    input_level_meter_bits: Arc<AtomicU32>,
    rider_gain_meter_bits: Arc<AtomicU32>,
    denoise_gain_meter_bits: Arc<AtomicU32>,
}

#[derive(Params)]
struct SimpleVoiceCleanerParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[id = "denoise_enabled"]
    pub denoise_enabled: BoolParam,
    #[id = "denoise_amount"]
    pub denoise_amount: FloatParam,
    #[id = "denoise_floor"]
    pub denoise_floor_db: FloatParam,
    #[id = "denoise_softness"]
    pub denoise_softness_db: FloatParam,
    #[id = "hpf"]
    pub hpf_enabled: BoolParam,

    #[id = "target"]
    pub target_db: FloatParam,
    #[id = "amount"]
    pub ride_amount: FloatParam,
    #[id = "speed"]
    pub speed_ms: FloatParam,
    #[id = "floor"]
    pub noise_floor_db: FloatParam,
    #[id = "max_boost"]
    pub max_boost_db: FloatParam,
    #[id = "max_cut"]
    pub max_cut_db: FloatParam,
    #[id = "output"]
    pub output_gain_db: FloatParam,
    #[id = "limiter"]
    pub limiter: BoolParam,
}

impl Default for SimpleVoiceCleaner {
    fn default() -> Self {
        Self {
            params: Arc::new(SimpleVoiceCleanerParams::default()),
            sample_rate: 48_000.0,
            rms_state: 0.0,
            denoise_level_state: 0.0,
            rider_gain_db: 0.0,
            denoise_gain_db: 0.0,
            hpf_x1: [0.0; 2],
            hpf_y1: [0.0; 2],
            input_level_meter_bits: Arc::new(AtomicU32::new(MIN_DB.to_bits())),
            rider_gain_meter_bits: Arc::new(AtomicU32::new(0.0_f32.to_bits())),
            denoise_gain_meter_bits: Arc::new(AtomicU32::new(0.0_f32.to_bits())),
        }
    }
}

impl Default for SimpleVoiceCleanerParams {
    fn default() -> Self {
        let saved = load_settings().unwrap_or_default();
        Self {
            editor_state: EguiState::from_size(560, 340),
            denoise_enabled: BoolParam::new("Denoiser", saved.denoise_enabled),
            denoise_amount: FloatParam::new("Denoise Reduction", saved.denoise_reduction_db, FloatRange::Linear { min: 0.0, max: 100.0 }).with_unit(" dB").with_step_size(1.0),
            denoise_floor_db: FloatParam::new("Denoise Floor", saved.denoise_floor_db, FloatRange::Linear { min: -90.0, max: -25.0 }).with_unit(" dB").with_step_size(0.1),
            denoise_softness_db: FloatParam::new("Denoise Softness", saved.denoise_softness_db, FloatRange::Linear { min: 3.0, max: 30.0 }).with_unit(" dB").with_step_size(0.1),
            hpf_enabled: BoolParam::new("HPF 75Hz", saved.hpf_enabled),
            target_db: FloatParam::new("Target Level", saved.target_db, FloatRange::Linear { min: -36.0, max: -6.0 }).with_unit(" dB").with_step_size(0.1),
            ride_amount: FloatParam::new("Ride Amount", saved.ride_amount, FloatRange::Linear { min: 0.0, max: 100.0 }).with_unit("%").with_step_size(1.0),
            speed_ms: FloatParam::new("Speed", saved.speed_ms, FloatRange::Skewed { min: 80.0, max: 2000.0, factor: FloatRange::skew_factor(-2.0) }).with_unit(" ms").with_step_size(1.0),
            noise_floor_db: FloatParam::new("Noise Floor", saved.noise_floor_db, FloatRange::Linear { min: -80.0, max: -25.0 }).with_unit(" dB").with_step_size(0.1),
            max_boost_db: FloatParam::new("Max Boost", saved.max_boost_db, FloatRange::Linear { min: 0.0, max: 18.0 }).with_unit(" dB").with_step_size(0.1),
            max_cut_db: FloatParam::new("Max Cut", saved.max_cut_db, FloatRange::Linear { min: 0.0, max: 24.0 }).with_unit(" dB").with_step_size(0.1),
            output_gain_db: FloatParam::new("Output Gain", saved.output_gain_db, FloatRange::Linear { min: -24.0, max: 24.0 }).with_unit(" dB").with_step_size(0.1),
            limiter: BoolParam::new("Safety Limiter", saved.limiter),
        }
    }
}
impl Plugin for SimpleVoiceCleaner {
    const NAME: &'static str = "SimpleVoiceCleaner";
    const VENDOR: &'static str = "YourName";
    const URL: &'static str = "https://example.com";
    const EMAIL: &'static str = "you@example.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout { main_input_channels: NonZeroU32::new(2), main_output_channels: NonZeroU32::new(2), aux_input_ports: &[], aux_output_ports: &[], names: PortNames::const_default() },
        AudioIOLayout { main_input_channels: NonZeroU32::new(1), main_output_channels: NonZeroU32::new(1), aux_input_ports: &[], aux_output_ports: &[], names: PortNames::const_default() },
    ];

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> { self.params.clone() }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let input_meter = self.input_level_meter_bits.clone();
        let gain_meter = self.rider_gain_meter_bits.clone();
        let denoise_meter = self.denoise_gain_meter_bits.clone();

        create_egui_editor(
            params.editor_state.clone(),
            params.clone(),
            |_egui_ctx, _params| {},
            move |egui_ctx, setter, params| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    ui.set_min_size(egui::vec2(540.0, 320.0));
                    ui.vertical_centered(|ui| { ui.heading("SimpleVoiceCleaner"); });
                    ui.add_space(8.0);
                    let mut settings_changed = false;

                    let input_db = f32::from_bits(input_meter.load(Ordering::Relaxed));
                    let rider_db = f32::from_bits(gain_meter.load(Ordering::Relaxed));
                    let denoise_db = f32::from_bits(denoise_meter.load(Ordering::Relaxed));
                    let target_db = params.target_db.value();

                    ui.label(format!("Input Level: {:.1} dB    Denoise: {:+.1} dB    Rider Gain: {:+.1} dB", input_db, denoise_db, rider_db));
                    if draw_target_slider_and_meter(ui, setter, &params.target_db, input_db, target_db) { settings_changed = true; }
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        let mut denoise_on = params.denoise_enabled.value();
                        if ui.checkbox(&mut denoise_on, "Denoiser").changed() {
                            setter.begin_set_parameter(&params.denoise_enabled);
                            setter.set_parameter(&params.denoise_enabled, denoise_on);
                            setter.end_set_parameter(&params.denoise_enabled);
                            settings_changed = true;
                        }
                        let mut hpf_on = params.hpf_enabled.value();
                        if ui.checkbox(&mut hpf_on, "HPF 75Hz").changed() {
                            setter.begin_set_parameter(&params.hpf_enabled);
                            setter.set_parameter(&params.hpf_enabled, hpf_on);
                            setter.end_set_parameter(&params.hpf_enabled);
                            settings_changed = true;
                        }
                    });
                    ui.columns(3, |columns| {
                        if param_slider(&mut columns[0], setter, &params.denoise_amount, 0.0..=100.0, "Denoise Reduction dB") { settings_changed = true; }
                        if param_slider(&mut columns[1], setter, &params.denoise_floor_db, -90.0..=-25.0, "Denoise Floor dB") { settings_changed = true; }
                        if param_slider(&mut columns[2], setter, &params.denoise_softness_db, 3.0..=30.0, "Denoise Softness dB") { settings_changed = true; }
                    });
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.columns(2, |columns| {
                        if param_slider(&mut columns[0], setter, &params.ride_amount, 0.0..=100.0, "Ride Amount") { settings_changed = true; }
                        if param_slider(&mut columns[1], setter, &params.speed_ms, 80.0..=2000.0, "Speed ms") { settings_changed = true; }
                    });
                    ui.columns(3, |columns| {
                        if param_slider(&mut columns[0], setter, &params.noise_floor_db, -80.0..=-25.0, "Rider Floor dB") { settings_changed = true; }
                        if param_slider(&mut columns[1], setter, &params.max_boost_db, 0.0..=18.0, "Max Boost dB") { settings_changed = true; }
                        if param_slider(&mut columns[2], setter, &params.max_cut_db, 0.0..=24.0, "Max Cut dB") { settings_changed = true; }
                    });
                    ui.horizontal(|ui| {
                        if param_slider(ui, setter, &params.output_gain_db, -24.0..=24.0, "Output Gain dB") { settings_changed = true; }
                        let mut limiter = params.limiter.value();
                        if ui.checkbox(&mut limiter, "Safety Limiter").changed() {
                            setter.begin_set_parameter(&params.limiter);
                            setter.set_parameter(&params.limiter, limiter);
                            setter.end_set_parameter(&params.limiter);
                            settings_changed = true;
                        }
                    });
                    if settings_changed {
                        save_current_settings(&params);
                    }
                });
            },
        )
    }

    fn initialize(&mut self, _audio_io_layout: &AudioIOLayout, buffer_config: &BufferConfig, _context: &mut impl InitContext<Self>) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        true
    }

    fn reset(&mut self) {
        self.rms_state = 0.0;
        self.denoise_level_state = 0.0;
        self.rider_gain_db = 0.0;
        self.denoise_gain_db = 0.0;
        self.hpf_x1 = [0.0; 2];
        self.hpf_y1 = [0.0; 2];
        self.input_level_meter_bits.store(MIN_DB.to_bits(), Ordering::Relaxed);
        self.rider_gain_meter_bits.store(0.0_f32.to_bits(), Ordering::Relaxed);
        self.denoise_gain_meter_bits.store(0.0_f32.to_bits(), Ordering::Relaxed);
    }

    fn process(&mut self, buffer: &mut Buffer, _aux: &mut AuxiliaryBuffers, _context: &mut impl ProcessContext<Self>) -> ProcessStatus {
        let detector_coeff = coeff_from_tau(0.050, self.sample_rate);
        let denoise_detector_coeff = coeff_from_tau(0.025, self.sample_rate);
        let denoise_attack_coeff = coeff_from_tau(0.035, self.sample_rate);
        let denoise_release_coeff = coeff_from_tau(0.012, self.sample_rate);
        let speed_sec = self.params.speed_ms.value() * 0.001;
        let ride_coeff = coeff_from_tau(speed_sec.max(0.001), self.sample_rate);
        let silence_return_coeff = coeff_from_tau(0.900, self.sample_rate);

        let denoise_enabled = self.params.denoise_enabled.value();
        let hpf_enabled = self.params.hpf_enabled.value();
        let max_denoise_reduction_db = self.params.denoise_amount.value().clamp(0.0, 100.0);
        let denoise_enabled_and_active = denoise_enabled && max_denoise_reduction_db > 0.0;
        let denoise_floor_db = self.params.denoise_floor_db.value();
        let denoise_softness_db = self.params.denoise_softness_db.value().max(0.001);
        let hpf_alpha = highpass_alpha(75.0, self.sample_rate);

        let amount = self.params.ride_amount.value() / 100.0;
        let target_db = self.params.target_db.value();
        let floor_db = self.params.noise_floor_db.value();
        let max_boost_db = self.params.max_boost_db.value();
        let max_cut_db = self.params.max_cut_db.value();
        let output_gain = db_to_gain(self.params.output_gain_db.value());
        let limiter_enabled = self.params.limiter.value();
        let ceiling = db_to_gain(-1.0);

        for mut channel_samples in buffer.iter_samples() {
            let channel_count = channel_samples.len().max(1) as f32;
            let mut mono = 0.0_f32;

            // Stage 1: light denoiser pre-processing. This is a real-time adaptive expander,
            // not an AI separator. It sits before the vocal rider so the rider does not lift room tone.
            for (ch, sample) in channel_samples.iter_mut().enumerate() {
                let mut x = *sample;
                if hpf_enabled {
                    let idx = ch.min(1);
                    let y = hpf_alpha * (self.hpf_y1[idx] + x - self.hpf_x1[idx]);
                    self.hpf_x1[idx] = x;
                    self.hpf_y1[idx] = y;
                    x = y;
                    *sample = x;
                }
                mono += x;
            }
            mono /= channel_count;

            self.denoise_level_state = denoise_detector_coeff * self.denoise_level_state + (1.0 - denoise_detector_coeff) * mono * mono;
            let denoise_level_db = amp_to_db(self.denoise_level_state.sqrt());
            let target_denoise_gain_db = if denoise_enabled_and_active {
                let t = ((denoise_level_db - denoise_floor_db) / denoise_softness_db).clamp(0.0, 1.0);
                let smooth_t = t * t * (3.0 - 2.0 * t);
                -max_denoise_reduction_db * (1.0 - smooth_t)
            } else { 0.0 };

            let denoise_coeff = if target_denoise_gain_db < self.denoise_gain_db { denoise_attack_coeff } else { denoise_release_coeff };
            self.denoise_gain_db = denoise_coeff * self.denoise_gain_db + (1.0 - denoise_coeff) * target_denoise_gain_db;
            let denoise_gain = db_to_gain(self.denoise_gain_db);

            for sample in channel_samples.iter_mut() { *sample *= denoise_gain; }

            // Stage 2: vocal rider, measured after denoising.
            let post_denoise_mono = mono * denoise_gain;
            self.rms_state = detector_coeff * self.rms_state + (1.0 - detector_coeff) * post_denoise_mono * post_denoise_mono;
            let level_db = amp_to_db(self.rms_state.sqrt());
            let voice_active = level_db > floor_db;

            let target_gain_db = if voice_active {
                ((target_db - level_db) * amount).clamp(-max_cut_db, max_boost_db)
            } else { 0.0 };

            let coeff = if voice_active { ride_coeff } else { silence_return_coeff };
            self.rider_gain_db = coeff * self.rider_gain_db + (1.0 - coeff) * target_gain_db;
            let rider_gain = db_to_gain(self.rider_gain_db) * output_gain;

            for sample in channel_samples.iter_mut() {
                let mut x = *sample * rider_gain;
                if limiter_enabled { x = x.clamp(-ceiling, ceiling); }
                *sample = x;
            }

            self.input_level_meter_bits.store(level_db.to_bits(), Ordering::Relaxed);
            self.rider_gain_meter_bits.store(self.rider_gain_db.to_bits(), Ordering::Relaxed);
            self.denoise_gain_meter_bits.store(self.denoise_gain_db.to_bits(), Ordering::Relaxed);
        }
        ProcessStatus::Normal
    }
}

impl ClapPlugin for SimpleVoiceCleaner {
    const CLAP_ID: &'static str = "com.yourname.simplevoicecleaner";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A real-time light denoiser and vocal leveler for speech and streaming.");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(<Self as Plugin>::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = Some(<Self as Plugin>::URL);
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Utility];
}

impl Vst3Plugin for SimpleVoiceCleaner {
    const VST3_CLASS_ID: [u8; 16] = *b"SVoiceCleaner001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_clap!(SimpleVoiceCleaner);
nih_export_vst3!(SimpleVoiceCleaner);

fn draw_target_slider_and_meter(ui: &mut egui::Ui, setter: &ParamSetter, target_param: &FloatParam, input_db: f32, target_db: f32) -> bool {
    let desired_size = egui::vec2(ui.available_width().min(500.0), 58.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    let min_db = -60.0;
    let max_db = 0.0;
    let meter_rect = egui::Rect::from_min_max(egui::pos2(rect.left() + 44.0, rect.top() + 26.0), egui::pos2(rect.right() - 18.0, rect.top() + 42.0));
    let track_rect = egui::Rect::from_min_max(egui::pos2(meter_rect.left(), rect.top() + 8.0), egui::pos2(meter_rect.right(), rect.top() + 18.0));

    painter.text(egui::pos2(rect.left(), rect.top() + 24.0), egui::Align2::LEFT_TOP, "Input", egui::FontId::proportional(12.0), egui::Color32::from_gray(190));
    painter.text(egui::pos2(rect.left(), rect.top() + 4.0), egui::Align2::LEFT_TOP, "Target", egui::FontId::proportional(12.0), egui::Color32::from_gray(190));
    painter.rect_filled(meter_rect, 2.0, egui::Color32::from_rgb(22, 25, 28));
    painter.rect_stroke(meter_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(70)), egui::StrokeKind::Inside);
    painter.rect_filled(track_rect, 2.0, egui::Color32::from_rgb(28, 31, 35));
    painter.rect_stroke(track_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(60)), egui::StrokeKind::Inside);

    let input_norm = db_to_norm(input_db, min_db, max_db);
    let fill_rect = egui::Rect::from_min_max(meter_rect.left_top(), egui::pos2(meter_rect.left() + meter_rect.width() * input_norm, meter_rect.bottom()));
    let fill_color = if input_db > -6.0 { egui::Color32::from_rgb(235, 82, 68) } else if input_db > -18.0 { egui::Color32::from_rgb(238, 207, 72) } else { egui::Color32::from_rgb(76, 202, 111) };
    painter.rect_filled(fill_rect, 2.0, fill_color);

    let target_norm = db_to_norm(target_db, min_db, max_db);
    let target_x = meter_rect.left() + meter_rect.width() * target_norm;
    painter.line_segment([egui::pos2(target_x, track_rect.top() - 3.0), egui::pos2(target_x, meter_rect.bottom() + 4.0)], egui::Stroke::new(1.0, egui::Color32::from_gray(230)));
    let handle_rect = egui::Rect::from_center_size(egui::pos2(target_x, track_rect.center().y), egui::vec2(16.0, 22.0));
    painter.rect_filled(handle_rect, 3.0, egui::Color32::from_rgb(196, 188, 178));
    painter.rect_stroke(handle_rect, 3.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(245, 225, 190)), egui::StrokeKind::Inside);
    painter.text(egui::pos2(meter_rect.right(), rect.top() + 44.0), egui::Align2::RIGHT_TOP, format!("{:.1} dB", input_db), egui::FontId::proportional(12.0), egui::Color32::from_gray(190));
    painter.text(egui::pos2(target_x, rect.top() + 0.0), egui::Align2::CENTER_TOP, format!("{:.1}", target_db), egui::FontId::proportional(11.0), egui::Color32::from_gray(220));

    if response.dragged() || response.clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let norm = ((pointer.x - meter_rect.left()) / meter_rect.width()).clamp(0.0, 1.0);
            let new_target = (min_db + norm * (max_db - min_db)).clamp(-36.0, -6.0);
            setter.begin_set_parameter(target_param);
            setter.set_parameter(target_param, new_target);
            setter.end_set_parameter(target_param);
            return true;
        }
    }
    false
}

fn param_slider(ui: &mut egui::Ui, setter: &ParamSetter, param: &FloatParam, range: std::ops::RangeInclusive<f32>, label: &str) -> bool {
    let mut changed = false;
    ui.vertical(|ui| {
        ui.label(label);
        let mut value = param.value();
        if ui.add(egui::Slider::new(&mut value, range)).changed() {
            setter.begin_set_parameter(param);
            setter.set_parameter(param, value);
            setter.end_set_parameter(param);
            changed = true;
        }
    });
    changed
}

fn settings_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))?;
    Some(base.join("SimpleVoiceCleaner").join("settings.txt"))
}

fn parse_bool(value: &str, fallback: bool) -> bool {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => true,
        "false" | "0" | "no" | "off" => false,
        _ => fallback,
    }
}

fn parse_f32(value: &str, fallback: f32, min: f32, max: f32) -> f32 {
    value.trim().parse::<f32>().unwrap_or(fallback).clamp(min, max)
}

fn load_settings() -> Option<SavedSettings> {
    let path = settings_path()?;
    let content = fs::read_to_string(path).ok()?;
    let mut saved = SavedSettings::default();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let Some((key, value)) = line.split_once('=') else { continue; };
        match key.trim() {
            "denoise_enabled" => saved.denoise_enabled = parse_bool(value, saved.denoise_enabled),
            "denoise_reduction_db" | "denoise_amount" => saved.denoise_reduction_db = parse_f32(value, saved.denoise_reduction_db, 0.0, 100.0),
            "denoise_floor_db" => saved.denoise_floor_db = parse_f32(value, saved.denoise_floor_db, -90.0, -25.0),
            "denoise_softness_db" => saved.denoise_softness_db = parse_f32(value, saved.denoise_softness_db, 3.0, 30.0),
            "hpf_enabled" => saved.hpf_enabled = parse_bool(value, saved.hpf_enabled),
            "target_db" => saved.target_db = parse_f32(value, saved.target_db, -36.0, -6.0),
            "ride_amount" => saved.ride_amount = parse_f32(value, saved.ride_amount, 0.0, 100.0),
            "speed_ms" => saved.speed_ms = parse_f32(value, saved.speed_ms, 80.0, 2000.0),
            "noise_floor_db" => saved.noise_floor_db = parse_f32(value, saved.noise_floor_db, -80.0, -25.0),
            "max_boost_db" => saved.max_boost_db = parse_f32(value, saved.max_boost_db, 0.0, 18.0),
            "max_cut_db" => saved.max_cut_db = parse_f32(value, saved.max_cut_db, 0.0, 24.0),
            "output_gain_db" => saved.output_gain_db = parse_f32(value, saved.output_gain_db, -24.0, 24.0),
            "limiter" => saved.limiter = parse_bool(value, saved.limiter),
            _ => {}
        }
    }

    Some(saved)
}

fn save_current_settings(params: &SimpleVoiceCleanerParams) {
    let Some(path) = settings_path() else { return; };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let content = format!(
        concat!(
            "# SimpleVoiceCleaner settings\n",
            "denoise_enabled={}\n",
            "denoise_reduction_db={}\n",
            "denoise_floor_db={}\n",
            "denoise_softness_db={}\n",
            "hpf_enabled={}\n",
            "target_db={}\n",
            "ride_amount={}\n",
            "speed_ms={}\n",
            "noise_floor_db={}\n",
            "max_boost_db={}\n",
            "max_cut_db={}\n",
            "output_gain_db={}\n",
            "limiter={}\n"
        ),
        params.denoise_enabled.value(),
        params.denoise_amount.value(),
        params.denoise_floor_db.value(),
        params.denoise_softness_db.value(),
        params.hpf_enabled.value(),
        params.target_db.value(),
        params.ride_amount.value(),
        params.speed_ms.value(),
        params.noise_floor_db.value(),
        params.max_boost_db.value(),
        params.max_cut_db.value(),
        params.output_gain_db.value(),
        params.limiter.value(),
    );

    let _ = fs::write(path, content);
}

#[inline] fn db_to_norm(db: f32, min_db: f32, max_db: f32) -> f32 { ((db.clamp(min_db, max_db) - min_db) / (max_db - min_db)).clamp(0.0, 1.0) }
#[inline] fn coeff_from_tau(tau_seconds: f32, sample_rate: f32) -> f32 { (-1.0 / (tau_seconds * sample_rate)).exp() }
#[inline] fn amp_to_db(amp: f32) -> f32 { if amp <= 0.000_001 { MIN_DB } else { 20.0 * amp.log10() } }
#[inline] fn db_to_gain(db: f32) -> f32 { 10.0_f32.powf(db / 20.0) }
#[inline]
fn highpass_alpha(cutoff_hz: f32, sample_rate: f32) -> f32 {
    let dt = 1.0 / sample_rate.max(1.0);
    let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff_hz.max(1.0));
    rc / (rc + dt)
}
