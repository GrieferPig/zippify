/**
 * Zippify VST Plugin: lil' simple distortion/clipper VST2 Effect
 * Part of a suite "Magic 5 VST FX Plugins"
 * Author: GrieferPig
 *
 * Processing chain:
 * 1. Remove silences
 * 2. Clamp waveform (clipping)
 * 3. Decrease precision
 * 5. Mix
 *
 * Notes:
 * It is suggested to use this plugin with a filter because this plugin will bring
 * unwanted extra frequencies.
 */

#[macro_use]
extern crate vst;

use vst::{editor::Editor, prelude::*};

use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

use std::sync::Arc;

mod param;
mod process;
mod ui;
mod util;

use crate::process::process;
use param::{EffectParams, PARAM_NUM};
use ui::PluginEditor;

struct Zippify {
    params: Arc<EffectParams>,
    editor: Option<PluginEditor>,
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
                params,
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
            parameters: PARAM_NUM, // num of param we have
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

        process(in_buf_l, in_buf_r, out_buf_l, out_buf_r, &self.params);
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

plugin_main!(Zippify); // Important!
