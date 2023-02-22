/**
 * Zippify VST Plugin: lil' simple distortion/clipper VST2 Effect
 * Part of a suite "Magic 5 VST FX Plugins"
 * Author: GrieferPig
 *
 * Processing chain:
 * 1. Remove silences
 * 2. Clamp waveform (clipping)
 * 3. Decrease precision
 * 4. Gain
 * 5. Mix
 *
 * Notes:
 * It is suggested to use this plugin with a filter because this plugin will bring
 * unwanted extra frequencies.
 */

#[macro_use]
extern crate vst;

use vst::{editor::Editor, prelude::*};

use baseview::{Size, WindowHandle, WindowOpenOptions, WindowScalePolicy};
use egui::{
    style::Margin, Color32, ColorImage, Context, FontData, FontDefinitions, FontFamily, FontId,
    Frame, RichText, TextureHandle, Vec2,
};
use egui_baseview::{EguiWindow, Queue};

use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

use std::{sync::Arc, sync::Mutex, time::Duration};

struct Zippify {
    params: Arc<EffectParams>,
    editor: Option<PluginEditor>,
}

/*
 * Constants
 */

const SILENT_THRESHOLD_DB: f32 = 0.015848931924611134;
const SILENT_THRESHOLD_COUNT: i32 = 128;

const WINDOW_WIDTH: usize = 600;
const WINDOW_HEIGHT: usize = 400;

/*
 * Util funcs
 */

fn load_image_from_memory(image_data: &[u8]) -> Result<ColorImage, image::ImageError> {
    let image = image::load_from_memory(image_data)?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
}

fn get_loudest_sample(list: &[f32]) -> f32 {
    let mut max_number = list[0];
    let mut elem_abs: f32;
    for &elem in list {
        elem_abs = elem.abs();
        if elem_abs > max_number {
            max_number = elem_abs;
        }
    }
    max_number
}

fn to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

fn to_db(linear: f32) -> f32 {
    20.0 * linear.log10()
}

/**
 * manipulating samples functions
 */

fn mix((in_l, in_r): (&[f32], &[f32]), (out_l, out_r): (&mut [f32], &mut [f32]), mix: f32) {
    // Mix it L
    for (out_buf_l_sample, in_buf_l_sample) in out_l.iter_mut().zip(in_l.iter()) {
        *out_buf_l_sample = (*out_buf_l_sample * mix) + ((1.0 - mix) * in_buf_l_sample);
    }

    // Mix it R
    for (out_buf_r_sample, in_buf_r_sample) in out_r.iter_mut().zip(in_r.iter()) {
        *out_buf_r_sample = (*out_buf_r_sample * mix) + ((1.0 - mix) * in_buf_r_sample);
    }
}

fn remove_silence((out_buf_l, out_buf_r): (&mut [f32], &mut [f32])) {
    // Set silence sample counter
    let mut silence_counter_l: i32 = 0;
    let mut silence_counter_r: i32 = 0;

    // Ignore silence if loudness < threshold L
    for out_buf_l_sample in &mut *out_buf_l {
        if *out_buf_l_sample < SILENT_THRESHOLD_DB {
            silence_counter_l += 1;
            if silence_counter_l > SILENT_THRESHOLD_COUNT {
                *out_buf_l_sample = 0.0;
                break;
            }
        }
        silence_counter_l = 0;
    }

    // Ignore silence if loudness < threshold R
    for out_buf_r_sample in &mut *out_buf_r {
        if *out_buf_r_sample < SILENT_THRESHOLD_DB {
            silence_counter_r += 1;
            if silence_counter_r > SILENT_THRESHOLD_COUNT {
                *out_buf_r_sample = 0.0;
                break;
            }
        }
        silence_counter_r = 0;
    }
}

/*
 * Declare and impl params
 * Use atomic types for thread safety
 */

struct EffectParams {
    clamp_threshold: AtomicFloat,
    lose_precision: AtomicFloat,
    mix: AtomicFloat,
}

impl Default for EffectParams {
    fn default() -> EffectParams {
        EffectParams {
            clamp_threshold: AtomicFloat::new(to_linear(-12.0)),
            lose_precision: AtomicFloat::new(1.0),
            mix: AtomicFloat::new(1.0),
        }
    }
}

impl PluginParameters for EffectParams {
    // getter
    fn get_parameter(&self, index: i32) -> f32 {
        match index {
            0 => self.clamp_threshold.get(),
            1 => self.lose_precision.get(),
            2 => self.mix.get(),
            _ => 0.0,
        }
    }

    // setter
    fn set_parameter(&self, index: i32, val: f32) {
        match index {
            0 => self.clamp_threshold.set(val),
            1 => self.lose_precision.set(val),
            2 => self.mix.set(val),
            _ => (),
        }
    }

    // shows formatted param
    fn get_parameter_text(&self, index: i32) -> String {
        match index {
            0 => format!("{:.2} dB", to_db(self.clamp_threshold.get())),
            1 => format!("{:.2}", self.lose_precision.get()),
            2 => format!("{:.2}", self.mix.get()),
            _ => "".to_string(),
        }
    }

    // shows the control's name.
    fn get_parameter_name(&self, index: i32) -> String {
        match index {
            0 => "Chocolate!",
            1 => "Add bright freq",
            2 => "Mix",
            _ => "",
        }
        .to_string()
    }
}

/**
 * Declare editer ui
 */

struct PluginEditor {
    params: Arc<EffectParams>,
    is_open: bool,
    window_handle: Option<WindowHandleNew>,
}

impl Editor for PluginEditor {
    fn size(&self) -> (i32, i32) {
        (WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32)
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn open(&mut self, parent: *mut std::os::raw::c_void) -> bool {
        if self.is_open {
            return false;
        }
        self.is_open = true;

        let settings = WindowOpenOptions {
            title: String::from("Zippify"),
            size: Size::new(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64),
            scale: WindowScalePolicy::SystemScaleFactor,
            gl_config: Some(Default::default()),
        };

        let side_image_texture: Arc<Mutex<Option<TextureHandle>>> = Arc::new(Mutex::new(None));
        let side_image_texture_build = side_image_texture.clone();
        let side_image_texture_update = side_image_texture.clone();

        let window_handle = EguiWindow::open_parented(
            &VstParent(parent),
            settings,
            self.params.clone(),
            // Called once before the first frame. Allows you to do setup code and to
            // call `ctx.set_fonts()`. Optional.
            move |_egui_ctx: &Context, _queue: &mut Queue, _state: &mut Arc<EffectParams>| {
                // set to light mode
                _egui_ctx.set_visuals(egui::Visuals::light());
                // load custom font
                let mut fonts = FontDefinitions::default();
                fonts.font_data.insert(
                    "Adventure".to_owned(),
                    FontData::from_static(include_bytes!("./res/Adventure.ttf")),
                );
                fonts.font_data.insert(
                    "RobotoLight".to_owned(),
                    FontData::from_static(include_bytes!("./res/Roboto-Light.ttf")),
                );
                fonts.font_data.insert(
                    "Roboto".to_owned(),
                    FontData::from_static(include_bytes!("./res/Roboto-Regular.ttf")),
                );
                // put at first priority
                fonts
                    .families
                    .get_mut(&FontFamily::Monospace)
                    .unwrap()
                    .insert(0, "Adventure".to_owned());
                fonts
                    .families
                    .get_mut(&FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "Roboto".to_owned());
                fonts.families.insert(
                    FontFamily::Name("RobotoLight".into()),
                    Vec::from(["RobotoLight".to_owned()]),
                );
                _egui_ctx.set_fonts(fonts);
                let mut image_tex = side_image_texture_build.lock().unwrap();
                *image_tex = Some(
                    _egui_ctx.load_texture(
                        "side-image",
                        load_image_from_memory(include_bytes!(
                            "./res/did_somepony_say_chocolate.jpg"
                        ))
                        .unwrap(),
                        egui::TextureFilter::Linear,
                    ),
                );

                // first frame's ui is broken, request a repaint to fix
                _egui_ctx.request_repaint();
            },
            // Called before each frame. Here you should update the state of your
            // application and build the UI.
            move |egui_ctx: &Context, _queue: &mut Queue, state: &mut Arc<EffectParams>| {
                egui::SidePanel::right("image-panel")
                    .frame(Frame {
                        inner_margin: Margin {
                            top: 85.0,
                            right: 40.0,
                            ..Default::default()
                        },
                        fill: Color32::from_rgb(248, 248, 248),
                        ..Default::default()
                    })
                    .resizable(false)
                    .show(&egui_ctx, |ui| {
                        ui.image(
                            (*side_image_texture_update.lock().unwrap())
                                .as_ref()
                                .unwrap(),
                            Vec2::new(220.0, 220.0),
                        );
                        ui.hyperlink_to(
                            "Image by Galaxy Swirl",
                            "https://derpibooru.org/profiles/GalaxYSwiRL45",
                        )
                    });
                egui::CentralPanel::default().show(&egui_ctx, |_ui| {
                    egui::TopBottomPanel::top("top_panel")
                        .frame(Frame {
                            inner_margin: Margin {
                                top: 25.0,
                                left: 40.0,
                                bottom: 25.0,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .show(&egui_ctx, |ui| {
                            ui.heading(
                                RichText::new("Zippify")
                                    .font(FontId {
                                        size: 60.0,
                                        family: FontFamily::Monospace,
                                    })
                                    .color(Color32::from_rgb(255, 107, 183)),
                            );
                            ui.label(
                                RichText::new("lil' simple distortion/clipper plugin")
                                    .color(Color32::from_rgba_premultiplied(255, 107, 183, 204))
                                    .font(FontId {
                                        size: 16.0,
                                        family: FontFamily::Name("RobotoLight".into()),
                                    }),
                            );
                        });
                    egui::TopBottomPanel::bottom("bottom_panel")
                        .frame(Frame {
                            inner_margin: Margin {
                                bottom: 25.0,
                                left: 40.0,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .show(&egui_ctx, |ui| {
                            ui.label(RichText::new("made by grieferpig").font(FontId {
                                size: 20.0,
                                family: FontFamily::Name("RobotoLight".into()),
                            }));
                        });
                    egui::CentralPanel::default()
                        .frame(Frame {
                            inner_margin: Margin {
                                left: 40.0,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .show(&egui_ctx, |ui| {
                            let mut clamp_threshold = state.clamp_threshold.get();
                            let mut is_lose_precision = state.lose_precision.get() > 0.5;
                            let mut mix = state.mix.get();

                            let clamp_slider_text = if clamp_threshold > 0.15 {
                                "Chocolate?"
                            } else if clamp_threshold > 0.02 {
                                "Chocolate!"
                            } else {
                                "CHOCOLATE!!!"
                            };
                            if ui
                                .add(
                                    egui::Slider::new(&mut clamp_threshold, 0.01..=1.0)
                                        .text(clamp_slider_text)
                                        .logarithmic(true),
                                )
                                .changed()
                            {
                                state.clamp_threshold.set(clamp_threshold)
                            }
                            ui.label(format!(
                                "Clamp threshold: {:.2} dB",
                                to_db(state.clamp_threshold.get())
                            ));
                            if ui
                                .add(egui::Checkbox::new(
                                    &mut is_lose_precision,
                                    "Add some bright frequencies",
                                ))
                                .changed()
                            {
                                state
                                    .lose_precision
                                    .set(if is_lose_precision { 1.0 } else { 0.0 })
                            }
                            if ui
                                .add(egui::Slider::new(&mut mix, 0.0..=1.0).text("mix"))
                                .changed()
                            {
                                state.mix.set(mix)
                            }
                            ui.label(format!("Mix: {:.2}%", state.mix.get() * 100.0));
                        })
                });
                // update per 200 ms to follow param changes
                egui_ctx.request_repaint_after(Duration::new(0, 200));
            },
        );

        self.window_handle = Some(WindowHandleNew {
            handle: window_handle,
        });

        true
    }

    fn is_open(&mut self) -> bool {
        self.is_open
    }

    fn close(&mut self) {
        self.is_open = false;
        match &mut self.window_handle {
            Some(x) => {
                x.handle.close();
                self.window_handle = None;
            }
            _ => {}
        }
    }
}
/*
 * Plugin impl
 */

impl Plugin for Zippify {
    fn new(_host: HostCallback) -> Self {
        let params = Arc::new(EffectParams::default());
        Zippify {
            params: params.clone(),
            editor: Some(PluginEditor {
                params: params.clone(),
                is_open: false,
                window_handle: None,
            }),
        }
    }

    fn get_info(&self) -> Info {
        Info {
            name: "Zippify".to_string(),
            unique_id: 0xdbef, // Used by hosts to differentiate between plugins.
            vendor: "GrieferPig".to_string(),
            inputs: 2,
            outputs: 2,
            category: Category::Effect,
            parameters: 3, // num of param we have
            ..Default::default()
        }
    }

    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        Arc::clone(&self.params) as Arc<dyn PluginParameters>
    }

    fn get_editor(&mut self) -> Option<Box<dyn Editor>> {
        if let Some(editor) = self.editor.take() {
            Some(Box::new(editor) as Box<dyn Editor>)
        } else {
            None
        }
    }

    // Note: In Ableton Live, there is no sample goes into the input buffer.
    // It is stored in the output buffer instead.
    // Other DAWs may still use the input buffer so it's necessary to check the input buffer first

    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        let (in_buf, mut out_buf) = buffer.split();

        // make this mutable in case the input buffer is not zero
        // let (mut in_buf_l, mut in_buf_r) = (*out_buf.get(0), *out_buf.get(1));
        let mut in_buf_l = (*out_buf.get(0)).to_vec();
        let mut in_buf_r = (*out_buf.get(1)).to_vec();

        let (out_buf_l, out_buf_r) = (out_buf.get_mut(0), out_buf.get_mut(1));

        // use orig input buf if not zero
        // only check 1st channel but maybe the data is in the 2nd one
        // but i don't think that's gonna happen
        for in_buf_l_sample in in_buf.get(0).iter() {
            if *in_buf_l_sample != 0.0 {
                in_buf_l = (*in_buf.get(0)).to_vec();
                in_buf_r = (*in_buf.get(1)).to_vec();
                break;
            }
        }

        // finalizing the input buf for security reasons
        let in_buf_l: &[f32] = &in_buf_l;
        let in_buf_r: &[f32] = &in_buf_r;

        // get param
        let clamp_range = self.params.clamp_threshold.get();
        let lose_precision = self.params.lose_precision.get();
        let mix_level = self.params.mix.get();

        // remove silence
        remove_silence((out_buf_l, out_buf_r));

        // Clamp L
        for (index, in_buf_l_sample) in in_buf_l.iter().enumerate() {
            out_buf_l[index] = in_buf_l_sample.clamp(-clamp_range, clamp_range);
        }

        // Clamp R
        for (index, in_buf_r_sample) in in_buf_r.iter().enumerate() {
            out_buf_r[index] = in_buf_r_sample.clamp(-clamp_range, clamp_range);
        }

        if lose_precision > 0.5 {
            // Lose precision L
            for out_buf_l_sample in &mut *out_buf_l {
                *out_buf_l_sample =
                    f32::from_be_bytes((out_buf_l_sample.to_bits() & 0xffff0000).to_be_bytes());
            }

            // Lose precision R
            for out_buf_r_sample in &mut *out_buf_r {
                *out_buf_r_sample =
                    f32::from_be_bytes((out_buf_r_sample.to_bits() & 0xffff0000).to_be_bytes());
            }
        }

        // Get the loudest sample from input, calc diff w/ CLAMP_RANGE
        let diff_l = get_loudest_sample(&in_buf_l) / clamp_range;
        let diff_r = get_loudest_sample(&in_buf_r) / clamp_range;

        // Gain to loudest sample L
        for out_buf_l_sample in &mut *out_buf_l {
            *out_buf_l_sample *= diff_l;
        }

        // Gain to loudest sample R
        for out_buf_r_sample in &mut *out_buf_r {
            *out_buf_r_sample *= diff_r;
        }

        // Mix
        mix((in_buf_l, in_buf_r), (out_buf_l, out_buf_r), mix_level);
    }
}

struct VstParent(*mut ::std::ffi::c_void);

#[cfg(target_os = "macos")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::macos::MacOSHandle;

        RawWindowHandle::MacOS(MacOSHandle {
            ns_view: self.0 as *mut ::std::ffi::c_void,
            ..MacOSHandle::empty()
        })
    }
}

#[cfg(target_os = "windows")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::Win32Handle;

        let mut handle = Win32Handle::empty();
        handle.hwnd = self.0;
        RawWindowHandle::Win32(handle)
    }
}

#[cfg(target_os = "linux")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::unix::XcbHandle;

        RawWindowHandle::Xcb(XcbHandle {
            window: self.0 as u32,
            ..XcbHandle::empty()
        })
    }
}

struct WindowHandleNew {
    handle: WindowHandle,
}

unsafe impl Send for WindowHandleNew {}
unsafe impl Sync for WindowHandleNew {}

plugin_main!(Zippify); // Important!
