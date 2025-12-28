use std::fmt;
use std::time::Duration;

use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::boundary::Boundary;
use crate::camera::{
	worldspace_to_screenspace,
	WORLD_TO_PIXELS
};
use crate::keyboard::Keyboard;

pub const SIZE: f64 = 1.0;
pub const SPEED: f64 = 24.0;

pub struct Helicopter {
	pub x: f64,
	pub y: f64,
	pub is_dead: bool
}

impl Helicopter {
	pub fn new(x: f64, y: f64) -> Self {
		Self {
			x,
			y,
			is_dead: false
		}
	}

	pub fn update(&mut self, delta_time: &Duration, keyboard: &Keyboard, boundaries: &Vec<Boundary>) {
		let delta_time = delta_time.as_millis() as f64 / 1000.0;
		let mut delta_y = 0.0;
		let mut delta_x = 0.0;

		if keyboard.is_w_down {
			delta_y = -SPEED * delta_time;
		} else if keyboard.is_s_down {
			delta_y = SPEED * delta_time;
		}

		if keyboard.is_a_down {
			delta_x = -SPEED * delta_time;
		} else if keyboard.is_d_down {
			delta_x = SPEED * delta_time;
		}

		if (keyboard.is_w_down || keyboard.is_s_down) && (keyboard.is_a_down || keyboard.is_d_down) {
			delta_x /= 2.0_f64.sqrt();
			delta_y /= 2.0_f64.sqrt();
		}

		for boundary in boundaries.iter() {
			if !(self.x + delta_x >= boundary.x + boundary.width || self.x + delta_x + SIZE <= boundary.x || self.y >= boundary.y + boundary.height || self.y + SIZE <= boundary.y) {
				if self.x >= boundary.x + boundary.width {
					delta_x = boundary.x + boundary.width - self.x;
				} else {
					delta_x = boundary.x - SIZE - self.x;
				}
			}

			if !(self.x >= boundary.x + boundary.width || self.x + SIZE <= boundary.x || self.y + delta_y >= boundary.y + boundary.height || self.y + delta_y + SIZE <= boundary.y) {
				if self.y >= boundary.y + boundary.height {
					delta_y = boundary.y + boundary.height - self.y;
				} else {
					delta_y = boundary.y - SIZE - self.y;
				}
			}
		}

		self.x += delta_x;
		self.y += delta_y;
	}

	pub fn draw(&self, focus: (f64, f64), canvas: &mut Canvas<Window>) -> Result<(), String> {
		let (x, y) = worldspace_to_screenspace(focus, (self.x, self.y), canvas.window().size());
		let size = (SIZE * WORLD_TO_PIXELS) as u32;
		canvas.fill_rect(Rect::new(x, y, size, size))?;

		Ok(())
	}
}

impl fmt::Debug for Helicopter {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "HELICOPTER {} {} {}", self.x, self.y, self.is_dead)
	}
}
