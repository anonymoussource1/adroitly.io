use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::rect::Point;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::surface::Surface;
use sdl2::pixels::PixelFormatEnum;

use crate::camera::{
	worldspace_to_screenspace,
	WORLD_TO_PIXELS
};

pub const SIZE: f64 = 0.75;
pub const CONNECTION_HEIGHT: f64 = 0.5;

#[derive(Clone)]
pub struct Fort {
	pub x: f64,
	pub y: f64,
    pub connections: Vec<(f64, f64)>
}

impl Fort {
	pub fn new(x: f64, y: f64) -> Self {
		Fort {
			x,
			y,
            connections: Vec::new()
		}
	}

	pub fn draw(&self, focus: (f64, f64), canvas: &mut Canvas<Window>) -> Result<(), String> {
		let (x, y) = worldspace_to_screenspace(focus, (self.x, self.y), canvas.window().size());
		let size = (SIZE * WORLD_TO_PIXELS) as u32;
		canvas.fill_rect(Rect::new(x, y, size, size))?;

        Ok(())
    }

    pub fn draw_line(&self, focus: (f64, f64), canvas: &mut Canvas<Window>, color: Color) -> Result<(), String> {
		let (x, y) = worldspace_to_screenspace(focus, (self.x, self.y), canvas.window().size());

        for connection in &self.connections {
            let distance = ((connection.1 - self.y) * (connection.1 - self.y) + (connection.0 - self.x) * (connection.0 - self.x)).sqrt().abs();

            if distance as u32 == 0 { return Ok(()) }

            let temp_angle = ((connection.0 - self.x) / distance).acos() * (180.0 / std::f64::consts::PI);
            let angle = if (connection.1 - self.y) < 0.0 { -temp_angle } else { temp_angle };

            let texture_creator = canvas.texture_creator();
            let mut surface = Surface::new((distance * WORLD_TO_PIXELS) as u32, (CONNECTION_HEIGHT * WORLD_TO_PIXELS) as u32, PixelFormatEnum::RGB24)?;

            surface.fill_rect(Rect::new(0, 0, (distance * WORLD_TO_PIXELS) as u32, (CONNECTION_HEIGHT * WORLD_TO_PIXELS) as u32), color)?;

            let texture = surface.as_texture(&texture_creator).unwrap();

            canvas.copy_ex(&texture, None, Some(Rect::new(x + (SIZE / 2.0 * WORLD_TO_PIXELS) as i32, y + ((SIZE / 2.0 - 0.25) * WORLD_TO_PIXELS) as i32, (distance * WORLD_TO_PIXELS) as u32, (CONNECTION_HEIGHT * WORLD_TO_PIXELS) as u32)), angle, Some(Point::new(0, (0.25 * WORLD_TO_PIXELS) as i32)), false, false)?;
        }

		Ok(())
	}
}
