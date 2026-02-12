use std::fmt;

use rand::Rng;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::boundary::Boundary;
use crate::camera::{
	WORLD_TO_PIXELS,
	worldspace_to_screenspace
};
use crate::shapes::Rectangle;

pub const SIZE: f64 = 1.0;
pub const SPEED: f64 = 12.0;

pub struct Helicopter {
	pub x: f64,
	pub y: f64,
	pub dx: f64,
	pub dy: f64,
	pub is_dead: bool
}

impl Helicopter {
	pub fn new(x: f64, y: f64) -> Self {
		Self {
			x,
			y,
			dx: 0.0,
			dy: 0.0,
			is_dead: false
		}
	}

	pub fn draw(&self, focus: (f64, f64), canvas: &mut Canvas<Window>) -> Result<(), String> {
		let (x, y) = worldspace_to_screenspace(focus, (self.x, self.y), canvas.window().size());
		let size = (SIZE * WORLD_TO_PIXELS) as u32;
		canvas.fill_rect(Rect::new(x, y, size, size))?;

		Ok(())
	}

	pub fn bounds(&self) -> Rectangle {
		Rectangle::new(self.x, self.y, SIZE, SIZE)
	}
}

pub fn find_valid_spawn(boundaries: &Vec<Boundary>) -> (f64, f64) {
	'main: loop {
		let x = rand::rng().random_range(-50.0..=(50.0 - SIZE));
		let y = rand::rng().random_range(-50.0..=(50.0 - SIZE));
		let bounds = Rectangle::new(x, y, SIZE, SIZE);

		for boundary in boundaries.iter() {
			if bounds.intersects_with_segment(&boundary.bounds()).is_some() {
				continue 'main;
			}
		}

		return (x, y);
	}
}

impl fmt::Debug for Helicopter {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "HELICOPTER {} {} {}", self.x, self.y, self.is_dead)
	}
}
