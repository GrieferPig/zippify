use crate::param::EffectParams;

// The threshold below which to drop the signal
const SILENT_THRESHOLD_DB: f32 = 0.015_848_933;
const SILENT_THRESHOLD_COUNT: i32 = 32;

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
            }
        }
    }

    // Ignore silence if loudness < threshold R
    for out_buf_r_sample in &mut *out_buf_r {
        if *out_buf_r_sample < SILENT_THRESHOLD_DB {
            silence_counter_r += 1;
            if silence_counter_r > SILENT_THRESHOLD_COUNT {
                *out_buf_r_sample = 0.0;
            }
        }
    }
}

pub fn process(
    in_buf_l: &[f32],
    in_buf_r: &[f32],
    out_buf_l: &mut [f32],
    out_buf_r: &mut [f32],
    params: &EffectParams,
) {
    // get param
    let clamp_range = params.clamp_threshold.get();
    let lose_precision = params.lose_precision.get();
    let mix_level = params.mix.get();

    // remove silence
    // return early if silent to avoid unnecessary processing
    remove_silence((out_buf_l, out_buf_r));

    // Clamp L
    for (index, in_buf_l_sample) in in_buf_l.iter().enumerate() {
        out_buf_l[index] = in_buf_l_sample.clamp(-clamp_range, clamp_range);
    }

    // Clamp R
    for (index, in_buf_r_sample) in in_buf_r.iter().enumerate() {
        out_buf_r[index] = in_buf_r_sample.clamp(-clamp_range, clamp_range);
    }

    // gain
    for out_buf_l_sample in &mut *out_buf_l {
        *out_buf_l_sample *= params.gain.get();
    }

    for out_buf_r_sample in &mut *out_buf_r {
        *out_buf_r_sample *= params.gain.get();
    }

    // Lose precision
    if lose_precision > 0.5 {
        for out_buf_l_sample in &mut *out_buf_l {
            let sample = (*out_buf_l_sample * 0x0f as f32) as i8;
            *out_buf_l_sample = f32::from(sample) / 0x0f as f32;
        }

        for out_buf_r_sample in &mut *out_buf_r {
            let sample = (*out_buf_r_sample * 0x0f as f32) as i8;
            *out_buf_r_sample = f32::from(sample) / 0x0f as f32;
        }
    }

    // Mix
    mix((in_buf_l, in_buf_r), (out_buf_l, out_buf_r), mix_level);
}
