use sdl2;
use sdl2::Sdl;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::{ SystemTime, UNIX_EPOCH, Duration };
use std::thread;

mod helicopter;

use helicopter::Helicopter;

struct Keyboard {
    is_w_down: bool,
    is_a_down: bool,
    is_s_down: bool,
    is_d_down: bool,
    should_quit: bool,
}

impl Keyboard {
    fn new() -> Self {
        Keyboard {
            is_w_down: false,
            is_a_down: false,
            is_s_down: false,
            is_d_down: false,
            should_quit: false,
        }
    }
}

fn main() -> Result<(), String> {
    let sdl2_context = sdl2::init()?;
    let video_subsystem = sdl2_context.video()?;
    let window = video_subsystem
        .window("Adroitly.io", 1500, 1200)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window
        .into_canvas()
        .build()
        .map_err(|e| e.to_string())?;
    let mut keyboard = Keyboard::new();

    let mut heli = Helicopter::new(4.0, 6.0);

    canvas.set_draw_color(Color::RGB(20, 20, 20));
    canvas.clear();

    'main: loop {
        let start = get_current_time();

        get_input(&sdl2_context, &mut keyboard);

        if keyboard.should_quit {
            break 'main;
        }

        heli.update(&keyboard);

        canvas.set_draw_color(Color::RGB(20, 20, 20));
        canvas.clear();

        heli.draw(&mut canvas)?;

        canvas.present();

        thread::sleep(Duration::from_millis(1000 / 60) + get_current_time() - start);
    }

    Ok(())
}

fn get_input(sdl2_context: &Sdl, keyboard: &mut Keyboard) {
    let mut event_pump = match sdl2_context.event_pump() {
        Ok(event_pump) => event_pump,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. }
            | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => keyboard.should_quit = true,
            Event::KeyDown { keycode: Some(Keycode::W), .. } => keyboard.is_w_down = true,
            Event::KeyDown { keycode: Some(Keycode::A), .. } => keyboard.is_a_down = true,
            Event::KeyDown { keycode: Some(Keycode::S), .. } => keyboard.is_s_down = true,
            Event::KeyDown { keycode: Some(Keycode::D), .. } => keyboard.is_d_down = true,
            Event::KeyUp { keycode: Some(Keycode::W), .. } => keyboard.is_w_down = false,
            Event::KeyUp { keycode: Some(Keycode::A), .. } => keyboard.is_a_down = false,
            Event::KeyUp { keycode: Some(Keycode::S), .. } => keyboard.is_s_down = false,
            Event::KeyUp { keycode: Some(Keycode::D), .. } => keyboard.is_d_down = false,
            _ => (),
        }
    }
}


fn get_current_time() -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards")
}
