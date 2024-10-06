use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::time::Duration;

use crate::utils::Keyboard;

pub const SIZE: u32 = 20;

pub struct Helicopter {
    pub x: f64,
    pub y: f64,
    pub rot: f64,
    pub name: String,
}

impl Helicopter {
    pub fn new(x: i32, y: i32, name: String) -> Self {
        Self {
            x: x as f64,
            y: y as f64,
            rot: 0.0,
            name,
        }
    }

    pub fn update(&mut self, delta_time: &Duration, keyboard: &Keyboard) {
        let delta_time = delta_time.as_millis() as f64 / 1000.0;
        if keyboard.is_w_down {
            self.y -= 120.0 * delta_time;
        } else if keyboard.is_s_down {
            self.y += 120.0 * delta_time;
        }

        if keyboard.is_a_down {
            self.x -= 120.0 * delta_time;
        } else if keyboard.is_d_down {
            self.x += 120.0 * delta_time;
        }
    }

    pub fn draw(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        canvas.fill_rect(Rect::new(
            self.x as i32,
            self.y as i32,
            SIZE,
            SIZE,
        ))?;

        Ok(())
    }
}
