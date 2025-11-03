use std::time::{
	Duration,
	SystemTime,
	UNIX_EPOCH
};

pub const WORLD_TO_PIXELS: f64 = 20.0;

pub struct Keyboard {
	pub is_w_down: bool,
	pub is_a_down: bool,
	pub is_s_down: bool,
	pub is_d_down: bool,
	pub should_quit: bool
}

impl Keyboard {
	pub fn new() -> Self {
		Keyboard {
			is_w_down: false,
			is_a_down: false,
			is_s_down: false,
			is_d_down: false,
			should_quit: false
		}
	}
}

pub fn worldspace_to_screenspace(focus: (f64, f64), target: (f64, f64), window_size: (u32, u32)) -> (i32, i32) {
    let x = (focus.0 - target.0) * WORLD_TO_PIXELS;
    let y = (focus.1 - target.1) * WORLD_TO_PIXELS;
    (((window_size.0 / 2) as f64 - x) as i32, ((window_size.1 / 2) as f64 - y) as i32)
}

pub fn screenspace_to_worldspace(focus: (f64, f64), target: (i32, i32), window_size: (u32, u32)) -> (f64, f64) {
    let x = ((window_size.0 / 2) as i32 - target.0) as f64 / WORLD_TO_PIXELS as f64;
    let y = ((window_size.1 / 2) as i32 - target.1) as f64 / WORLD_TO_PIXELS as f64;
    (focus.0 - x, focus.1 - y)
}

pub fn get_current_time() -> Duration {
	SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards and we're all doomed anyway")
}
