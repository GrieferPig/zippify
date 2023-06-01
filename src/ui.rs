/**
 * Declare editer ui
 */
use baseview::{Size, WindowOpenOptions, WindowScalePolicy};
use std::{sync::Arc, sync::Mutex, time::Duration};

use egui::{
    style::Margin, Color32, ColorImage, Context, FontData, FontDefinitions, FontFamily, FontId,
    Frame, RichText, TextureHandle, Vec2,
};
use egui_baseview::{EguiWindow, Queue};

use vst::editor::Editor;

use crate::param::EffectParams;
use crate::util::WindowHandleNew;
use crate::util::{to_db, to_linear};
use crate::VstParent;

const WINDOW_WIDTH: usize = 600;
const WINDOW_HEIGHT: usize = 400;

pub struct PluginEditor {
    pub params: Arc<EffectParams>,
    pub is_open: bool,
    pub window_handle: Option<WindowHandleNew>,
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
        let side_image_texture_update = side_image_texture;

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
                    .show(egui_ctx, |ui| {
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
                egui::CentralPanel::default().show(egui_ctx, |_ui| {
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
                        .show(egui_ctx, |ui| {
                            ui.heading(
                                RichText::new("Zippify")
                                    .font(FontId {
                                        size: 60.0,
                                        family: FontFamily::Monospace,
                                    })
                                    .color(Color32::from_rgb(255, 107, 183)),
                            );
                            ui.label(
                                RichText::new("lil' & simple distortion/clipper plugin")
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
                        .show(egui_ctx, |ui| {
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
                        .show(egui_ctx, |ui| {
                            let mut clamp_threshold = state.clamp_threshold.get();
                            let mut is_lose_precision = state.lose_precision.get() > 0.5;
                            let mut mix = state.mix.get();
                            let mut gain = state.gain.get();

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
                                .add(egui::Checkbox::new(&mut is_lose_precision, "8-bitify"))
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
                            if ui
                                .add(
                                    egui::Slider::new(&mut gain, 1.0..=to_linear(24.53))
                                        .text("gain"),
                                )
                                .changed()
                            {
                                state.gain.set(gain)
                            }
                            ui.label(format!("Gain: {:.2} dB", to_db(state.gain.get())));
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
            None => {}
        }
    }
}

fn load_image_from_memory(image_data: &[u8]) -> Result<ColorImage, image::ImageError> {
    let image = image::load_from_memory(image_data)?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
}
