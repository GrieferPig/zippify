use baseview::WindowHandle;

pub fn to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

pub fn to_db(linear: f32) -> f32 {
    20.0 * linear.log10()
}

pub struct WindowHandleNew {
    pub handle: WindowHandle,
}

unsafe impl Send for WindowHandleNew {}
unsafe impl Sync for WindowHandleNew {}
