use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::camera::{
	WORLD_TO_PIXELS,
	worldspace_to_screenspace
};
use crate::shapes::{
	Rectangle,
	Segment,
	Vec2
};

pub const SIZE: f64 = 0.75;
pub const CONNECTION_HEIGHT: f64 = 0.5;
pub const MAX_HEALTH: u8 = 3;

#[derive(Clone)]
pub struct Fort {
	pub x: f64,
	pub y: f64,
	pub health: u8,
	pub connections: Vec<(f64, f64)>
}

impl Fort {
	pub fn new(x: f64, y: f64) -> Self {
		Fort {
			x,
			y,
			health: MAX_HEALTH,
			connections: Vec::new()
		}
	}

	pub fn draw(&self, focus: (f64, f64), canvas: &mut Canvas<Window>) -> Result<(), String> {
		let (x, y) = worldspace_to_screenspace(focus, (self.x, self.y), canvas.window().size());
		let size = (SIZE * WORLD_TO_PIXELS) as u32;
		canvas.fill_rect(Rect::new(x, y, size, size))?;

		Ok(())
	}

	pub fn draw_connections(&self, focus: (f64, f64), canvas: &mut Canvas<Window>, color: Color) -> Result<(), String> {
		for connection in &self.connections {
			let segment = Segment::new(
				Vec2::new(self.x + SIZE / 2.0, self.y + SIZE / 2.0),
				Vec2::new(connection.0 + SIZE / 2.0, connection.1 + SIZE / 2.0),
				CONNECTION_HEIGHT
			);

			segment.draw(focus, canvas, color, 255)?;
		}

		Ok(())
	}

	pub fn bounds(&self) -> Rectangle {
		Rectangle::new(self.x, self.y, SIZE, SIZE)
	}
}
