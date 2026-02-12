use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::shapes::{
	Vec2,
	Segment
};

pub struct Boundary {
	pub start: Vec2,
	pub end: Vec2,
	pub width: f64,
	pub color: Color
}

impl Boundary {
	pub fn new(start: Vec2, end: Vec2, width: f64, color: Color) -> Self {
		Boundary {
			start,
			end,
			width,
			color
		}
	}

	pub fn bounds(&self) -> Segment {
		Segment::new(self.start.clone(), self.end.clone(), self.width)
	}

	pub fn draw(&self, focus: (f64, f64), canvas: &mut Canvas<Window>) -> Result<(), String> {
		self.bounds().draw(focus, canvas, self.color, 255)?;
		
		Ok(())
	}
}
