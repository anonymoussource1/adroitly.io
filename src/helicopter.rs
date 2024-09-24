use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::utils::Keyboard;

const HELICOPTER_SIZE: u32 = 20;

pub struct Helicopter {
    pub x: f64,
    pub y: f64,
    pub dx: f64,
    pub dy: f64,
    pub rot: f64,
    pub name: String,
}

impl Helicopter {
    pub fn new(x: f64, y: f64, name: String) -> Self {
        Self {
            x,
            y,
            dx: 0.0,
            dy: 0.0,
            rot: 0.0,
            name,
        }
    }

    pub fn update(&mut self, keyboard: &Keyboard) {
        if keyboard.is_w_down {
            self.dy = -2.0;
        } else if keyboard.is_s_down {
            self.dy = 2.0;
        } else {
            self.dy = 0.0;
        }

        if keyboard.is_a_down {
            self.dx = -2.0;
        } else if keyboard.is_d_down {
            self.dx = 2.0;
        } else {
            self.dx = 0.0;
        }

        self.x += self.dx;
        self.y += self.dy;
    }

    pub fn draw(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        //canvas.set_draw_color(Color::RGB(225, 100, 100));
        canvas.fill_rect(Rect::new(
            self.x as i32,
            self.y as i32,
            HELICOPTER_SIZE,
            HELICOPTER_SIZE,
        ))?;

        Ok(())
    }
}
