use std::time::Duration;

use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::utils::{
    Keyboard,
    worldspace_to_screenspace,
    WORLD_TO_PIXELS
};

pub const SIZE: f64 = 1.0;

pub struct Helicopter {
	pub x: f64,
	pub y: f64,
	pub rot: f64,
	pub name: String
}

impl Helicopter {
	pub fn new(x: f64, y: f64, name: String) -> Self {
		Self {
			x: x,
			y: y,
			rot: 0.0,
			name
		}
	}

	pub fn update(&mut self, delta_time: &Duration, keyboard: &Keyboard) {
		let delta_time = delta_time.as_millis() as f64 / 1000.0;
		if keyboard.is_w_down {
			self.y -= 6.0 * delta_time;
		} else if keyboard.is_s_down {
			self.y += 6.0 * delta_time;
		}

		if keyboard.is_a_down {
			self.x -= 6.0 * delta_time;
		} else if keyboard.is_d_down {
			self.x += 6.0 * delta_time;
		}
	}

	pub fn draw(&self, focus: (f64, f64), canvas: &mut Canvas<Window>) -> Result<(), String> {
        let (x, y) = worldspace_to_screenspace(focus, (self.x, self.y), canvas.window().size());
        let size = (SIZE * WORLD_TO_PIXELS) as u32;
		canvas.fill_rect(Rect::new(x, y, size, size))?;

		Ok(())
	}
}
