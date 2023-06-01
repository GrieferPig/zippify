use vst::prelude::PluginParameters;
use vst::util::AtomicFloat;

// import functions from util.rs
use crate::util::{to_db, to_linear};

/*
 * Declare and impl params
 * Use atomic types for thread safety
 */

pub struct EffectParams {
    pub clamp_threshold: AtomicFloat,
    pub lose_precision: AtomicFloat,
    pub mix: AtomicFloat,
    pub gain: AtomicFloat,
}

pub const PARAM_NUM: i32 = 4;

impl Default for EffectParams {
    fn default() -> EffectParams {
        EffectParams {
            clamp_threshold: AtomicFloat::new(to_linear(-12.0)),
            lose_precision: AtomicFloat::new(1.0),
            mix: AtomicFloat::new(1.0),
            gain: AtomicFloat::new(to_linear(0.0)),
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
            3 => {
                let gain = self.gain.get();
                (gain - 1.0) / to_linear(24.0)
            }
            _ => 0.0,
        }
    }

    // setter
    fn set_parameter(&self, index: i32, val: f32) {
        match index {
            0 => self.clamp_threshold.set(val),
            1 => self.lose_precision.set(val),
            2 => self.mix.set(val),
            3 => {
                let gain = val * to_linear(24.0) + 1.0;
                self.gain.set(gain);
            }
            _ => (),
        }
    }

    // shows formatted param
    fn get_parameter_text(&self, index: i32) -> String {
        match index {
            0 => format!("{:.2} dB", to_db(self.clamp_threshold.get())),
            1 => format!("{:.2}", self.lose_precision.get()),
            2 => format!("{:.2}", self.mix.get()),
            3 => format!("{:.2} dB", to_db(self.gain.get())),
            _ => "".to_string(),
        }
    }

    // shows the control's name.
    fn get_parameter_name(&self, index: i32) -> String {
        match index {
            0 => "Chocolate!",
            1 => "8-bitify",
            2 => "Mix",
            3 => "Gain",
            _ => "",
        }
        .to_string()
    }
}
