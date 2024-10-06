use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::utils::Keyboard;

pub const SIZE: u32 = 20;

pub struct Helicopter {
    pub x: i32,
    pub y: i32,
    pub rot: f64,
    pub name: String,
}

impl Helicopter {
    pub fn new(x: i32, y: i32, name: String) -> Self {
        Self {
            x,
            y,
            rot: 0.0,
            name,
        }
    }

    pub fn update(&mut self, keyboard: &Keyboard) {
        if keyboard.is_w_down {
            self.y -= 2;
        } else if keyboard.is_s_down {
            self.y += 2;
        }

        if keyboard.is_a_down {
            self.x -= 2;
        } else if keyboard.is_d_down {
            self.x += 2;
        }
    }

    pub fn draw(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        //canvas.set_draw_color(Color::RGB(225, 100, 100));
        canvas.fill_rect(Rect::new(
            self.x as i32,
            self.y as i32,
            SIZE,
            SIZE,
        ))?;

        Ok(())
    }
}
