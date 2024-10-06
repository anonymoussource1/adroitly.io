use sdl2::render::Canvas;
use sdl2::video::Window;

pub const RADIUS: u32 = 10;

pub struct Bullet {
    x: f64,
    y: f64,
    dx: f64,
    dy: f64,
}

impl Bullet {
    pub fn new(x: i32, y: i32, dx: f64, dy: f64) -> Self {
        Self {
            x: x as f64,
            y: y as f64,
            dx,
            dy
        }
    }

    pub fn draw(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        draw_circle(canvas, self.x as i32, self.y as i32, RADIUS)?;

        Ok(())
    }

    pub fn update(&mut self) {
        self.x += self.dx * 5.0;
        self.y += self.dy * 5.0;
    }
}

fn draw_circle(canvas: &mut Canvas<Window>, x: i32, y: i32, radius: u32) -> Result<(), String> {
    let mut t1 = radius as i32 / 16;
    let mut rel_x = radius as i32;
    let mut rel_y = 0;

    while rel_x >= rel_y {
        canvas.draw_point((rel_x + x, rel_y + y))?;
        canvas.draw_point((-rel_x + x, rel_y + y))?;
        canvas.draw_point((rel_x + x, -rel_y + y))?;
        canvas.draw_point((-rel_x + x, -rel_y + y))?;
        canvas.draw_point((rel_y + x, rel_x + y))?;
        canvas.draw_point((-rel_y + x, rel_x + y))?;
        canvas.draw_point((rel_y + x, -rel_x + y))?;
        canvas.draw_point((-rel_y + x, -rel_x + y))?;

        rel_y += 1;
        t1 = t1 + rel_y;

        let t2 = t1 - rel_x;

        if t2 >= 0 {
            t1 = t2;
            rel_x = rel_x - 1;
        }
    }

    Ok(())
}
