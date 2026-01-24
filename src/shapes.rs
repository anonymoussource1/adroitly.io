use sdl2::pixels::{
	Color,
	PixelFormatEnum
};
use sdl2::rect::{
	Point,
	Rect
};
use sdl2::render::{
	BlendMode,
	Canvas
};
use sdl2::surface::Surface;
use sdl2::video::Window;

use crate::camera::{
	WORLD_TO_PIXELS,
	worldspace_to_screenspace
};

pub struct Vec2 {
	pub x: f64,
	pub y: f64
}

pub struct Rectangle {
	pub x: f64,
	pub y: f64,
	pub width: f64,
	pub height: f64
}

pub struct Segment {
	pub start: Vec2,
	pub end: Vec2,
	pub width: f64
}

impl Vec2 {
	pub fn new(x: f64, y: f64) -> Self {
		Self {
			x,
			y
		}
	}

	pub fn counter_clockwise_with(&self, a: &Vec2, b: &Vec2) -> bool {
		(b.y - self.y) * (a.x - self.x) > (a.y - self.y) * (b.x - self.x)
	}
}

impl Rectangle {
	pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
		Self {
			x,
			y,
			width,
			height
		}
	}

	pub fn intersects_with_rect(&self, other: &Rectangle) -> bool {
		!(self.x > other.x + other.width || self.x + self.width < other.x || self.y > other.y + other.height || self.y + self.height < other.y)
	}

	pub fn intersects_with_segment(&self, other: &Segment) -> bool {
		let top = Segment::new(Vec2::new(self.x, self.y), Vec2::new(self.x + self.width, self.y), 0.0);
		let right = Segment::new(Vec2::new(self.x + self.width, self.y), Vec2::new(self.x + self.width, self.y + self.height), 0.0);
		let bottom = Segment::new(Vec2::new(self.x + self.width, self.y + self.height), Vec2::new(self.x, self.y + self.height), 0.0);
		let left = Segment::new(Vec2::new(self.x, self.y + self.height), Vec2::new(self.x, self.y), 0.0);

		other.intersects_with_segment(&top) || other.intersects_with_segment(&right) || other.intersects_with_segment(&bottom) || other.intersects_with_segment(&left)
	}
}

#[allow(unused)]
impl Segment {
	pub fn new(start: Vec2, end: Vec2, width: f64) -> Self {
		Self {
			start,
			end,
			width
		}
	}

	pub fn length(&self) -> f64 {
		((self.end.y - self.start.y) * (self.end.y - self.start.y) + (self.end.x - self.start.x) * (self.end.x - self.start.x)).sqrt()
	}

	pub fn get_normal(&self) -> Vec2 {
		let dx = self.end.x - self.start.x;
		let dy = self.end.y - self.start.y;
		let length = (dx * dx + dy * dy).sqrt();

		Vec2::new(-dy / length, dx / length)
	}

	pub fn intersects_with_segment(&self, other: &Segment) -> bool {
		let normal = self.get_normal();
		let self_top = Segment::new(
			Vec2::new(self.start.x - self.width / 2.0 * normal.x, self.start.y - self.width / 2.0 * normal.y),
			Vec2::new(self.end.x - self.width / 2.0 * normal.x, self.end.y - self.width / 2.0 * normal.y),
			0.0
		);
		let self_bottom = Segment::new(
			Vec2::new(self.start.x + self.width / 2.0 * normal.x, self.start.y + self.width / 2.0 * normal.y),
			Vec2::new(self.end.x + self.width / 2.0 * normal.x, self.end.y + self.width / 2.0 * normal.y),
			0.0
		);
		let other_top = Segment::new(
			Vec2::new(other.start.x - other.width / 2.0 * normal.x, other.start.y - other.width / 2.0 * normal.y),
			Vec2::new(other.end.x - other.width / 2.0 * normal.x, other.end.y - other.width / 2.0 * normal.y),
			0.0
		);
		let other_bottom = Segment::new(
			Vec2::new(other.start.x + other.width / 2.0 * normal.x, other.start.y + other.width / 2.0 * normal.y),
			Vec2::new(other.end.x + other.width / 2.0 * normal.x, other.end.y + other.width / 2.0 * normal.y),
			0.0
		);

		self_top.intersects_without_thickness(&other_top)
			|| self_top.intersects_without_thickness(&other_bottom)
			|| self_bottom.intersects_without_thickness(&other_top)
			|| self_bottom.intersects_without_thickness(&other_bottom)
	}

	pub fn intersects_with_rect(&self, other: &Rectangle) -> bool {
		let top = Segment::new(Vec2::new(other.x, other.y), Vec2::new(other.x + other.width, other.y), 0.0);
		let right = Segment::new(Vec2::new(other.x + other.width, other.y), Vec2::new(other.x + other.width, other.y + other.height), 0.0);
		let bottom = Segment::new(Vec2::new(other.x + other.width, other.y + other.height), Vec2::new(other.x, other.y + other.height), 0.0);
		let left = Segment::new(Vec2::new(other.x, other.y + other.height), Vec2::new(other.x, other.y), 0.0);

		self.intersects_with_segment(&top) || self.intersects_with_segment(&right) || self.intersects_with_segment(&bottom) || self.intersects_with_segment(&left)
	}

	fn intersects_without_thickness(&self, other: &Segment) -> bool {
		self.start.counter_clockwise_with(&other.start, &other.end) != self.end.counter_clockwise_with(&other.start, &other.end)
			&& self.start.counter_clockwise_with(&self.end, &other.start) != self.start.counter_clockwise_with(&self.end, &other.end)
	}

	pub fn draw(&self, focus: (f64, f64), canvas: &mut Canvas<Window>, color: Color, transparency: u8) -> Result<(), String> {
		let (x, y) = worldspace_to_screenspace(focus, (self.start.x, self.start.y), canvas.window().size());

		let temp_angle = ((self.end.y - self.start.y) / (self.end.x - self.start.x)).atan() * (180.0 / std::f64::consts::PI);
		let angle = if self.end.x - self.start.x < 0.0 { temp_angle + 180.0 } else { temp_angle };

		let width = (self.length() * WORLD_TO_PIXELS) as u32;
		let height = (self.width * WORLD_TO_PIXELS) as u32;

		let mut surface = Surface::new(width, height, PixelFormatEnum::RGB24)?;
		surface.fill_rect(Rect::new(0, 0, width, height), color)?;

		let texture_creator = canvas.texture_creator();
		if let Ok(mut texture) = surface.as_texture(&texture_creator) {
			if transparency < 255 {
				texture.set_blend_mode(BlendMode::Blend);
				texture.set_alpha_mod(transparency);
			}

			canvas.copy_ex(
				&texture,
				None,
				Some(Rect::new(x, y - (self.width / 2.0 * WORLD_TO_PIXELS) as i32, width, height)),
				angle,
				Some(Point::new(0, (self.width / 2.0 * WORLD_TO_PIXELS) as i32)),
				false,
				false
			)?;
		} else { }

		Ok(())
	}
}
