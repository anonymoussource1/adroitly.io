use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::camera::{
	worldspace_to_screenspace,
	WORLD_TO_PIXELS
};

pub struct Boundary {
	pub x: f64,
	pub y: f64,
	pub width: f64,
	pub height: f64
}

impl Boundary {
	pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
		Boundary {
			x,
			y,
			width,
			height
		}
	}

	pub fn draw(&self, focus: (f64, f64), canvas: &mut Canvas<Window>) -> Result<(), String> {
		let (x, y) = worldspace_to_screenspace(focus, (self.x, self.y), canvas.window().size());
		let width = (self.width * WORLD_TO_PIXELS) as u32;
		let height = (self.height * WORLD_TO_PIXELS) as u32;
		canvas.set_draw_color(Color::RGB(60, 60, 60));
		canvas.fill_rect(Rect::new(x, y, width, height))?;

		Ok(())
	}
}
