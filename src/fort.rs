use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::camera::{
	worldspace_to_screenspace,
	WORLD_TO_PIXELS
};

const SIZE: f64 = 0.75;

pub struct Fort {
	pub x: f64,
	pub y: f64
}

impl Fort {
	pub fn new(x: f64, y: f64) -> Self {
		Fort {
			x,
			y
		}
	}

	pub fn draw(&self, focus: (f64, f64), canvas: &mut Canvas<Window>) -> Result<(), String> {
		let (x, y) = worldspace_to_screenspace(focus, (self.x, self.y), canvas.window().size());
		let size = (SIZE * WORLD_TO_PIXELS) as u32;
		canvas.fill_rect(Rect::new(x, y, size, size))?;

		Ok(())
	}
}
