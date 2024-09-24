use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::Sdl;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod helicopter;
mod network;
mod utils;

use helicopter::Helicopter;
use network::Network;
use utils::{get_current_time, Keyboard};

fn main() -> Result<(), String> {
    let sdl2_context = sdl2::init()?;
    let video_subsystem = sdl2_context.video()?;
    let window = video_subsystem
        .window("Adroitly.io", 1500, 1200)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let mut keyboard = Keyboard::new();
    let network = prompt_for_network();
    let mut heli = Helicopter::new(
        4.0,
        6.0,
        network
            .lock()
            .expect("Failed to acquire lock on network")
            .ip
            .clone(),
    );

    canvas.set_draw_color(Color::RGB(20, 20, 20));
    canvas.clear();

    'main: loop {
        let start = get_current_time();

        get_input(&sdl2_context, &mut keyboard);

        if keyboard.should_quit {
            break 'main;
        }

        heli.update(&keyboard);

        let mut network = network.lock().expect("Failed to acquire lock on network");
        network.send_pos(&heli);

        canvas.set_draw_color(Color::RGB(20, 20, 20));
        canvas.clear();

        canvas.set_draw_color(Color::RGB(225, 100, 100));
        heli.draw(&mut canvas)?;

        for heli in network.helis.values() {
            canvas.set_draw_color(Color::RGB(100, 100, 225));
            heli.lock()
                .expect("Failed to acquire lock on helicopter")
                .draw(&mut canvas)?;
        }

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
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => keyboard.should_quit = true,
            Event::KeyDown {
                keycode: Some(Keycode::W),
                ..
            } => keyboard.is_w_down = true,
            Event::KeyDown {
                keycode: Some(Keycode::A),
                ..
            } => keyboard.is_a_down = true,
            Event::KeyDown {
                keycode: Some(Keycode::S),
                ..
            } => keyboard.is_s_down = true,
            Event::KeyDown {
                keycode: Some(Keycode::D),
                ..
            } => keyboard.is_d_down = true,
            Event::KeyUp {
                keycode: Some(Keycode::W),
                ..
            } => keyboard.is_w_down = false,
            Event::KeyUp {
                keycode: Some(Keycode::A),
                ..
            } => keyboard.is_a_down = false,
            Event::KeyUp {
                keycode: Some(Keycode::S),
                ..
            } => keyboard.is_s_down = false,
            Event::KeyUp {
                keycode: Some(Keycode::D),
                ..
            } => keyboard.is_d_down = false,
            _ => (),
        }
    }
}

fn prompt_for_network() -> Arc<Mutex<Network>> {
    let mut response = String::new();

    println!("Welcome to Adroitly.io! Would you like to...");
    println!("  [1] Create a game");
    println!("  [2] Join a friend");

    io::stdin()
        .read_line(&mut response)
        .expect("Failed to read input from player");

    match response.as_str().trim() {
        "1" => create_game(),
        "2" => connect_to_game(),
        _ => panic!("Incorrect response!"),
    }
}

fn create_game() -> Arc<Mutex<Network>> {
    let network = Arc::new(Mutex::new(Network::new(&get_player_ip())));

    let network_clone = network.clone();
    thread::spawn(move || network::start_listening_for_connection(network_clone));

    network
}

fn connect_to_game() -> Arc<Mutex<Network>> {
    let mut response = String::new();

    println!("What is one of the player's IP address?");

    io::stdin()
        .read_line(&mut response)
        .expect("Failed to read input from player");

    response = response.as_str().trim().to_string();

    let mut peer = TcpStream::connect(&response).expect(&format!(
        "Failed to connect to player at IP address \"{}\"",
        &response
    ));

    let ip = get_player_ip();
    let network = Arc::new(Mutex::new(Network::new(&ip)));

    peer.write_all(&format!("JOIN {}", &ip).as_bytes())
        .expect("Failed to write to player");
    println!("SENT {} MESSAGE: \"JOIN {}\"", response, &ip);

    let mut network_lock = network.lock().expect("Failed to acquire lock on network");

    let mut buffer = [0; 1024];
    match peer.read(&mut buffer) {
        Ok(0) => {
            panic!("ThIS sHOuLd NoT bE HaPPeNIng");
        }
        Ok(bytes_read) => {
            let raw_message = String::from_utf8_lossy(&buffer[..bytes_read]);
            let message: Vec<&str> = raw_message.split(" ").collect();

            match message[0] {
                "CURRPLAYERS" if message.len() != 1 => {
                    println!("  RECIEVED CURRPLAYERS COMMAND");

                    for part in &message[1..] {
                        let mut peer = TcpStream::connect(part)
                            .expect(&format!("Failed to connect to IP address {}", part));
                        peer.write_all(&format!("NEWPEER {}", &network_lock.ip).as_bytes())
                            .expect(&format!("Failed to write to IP address {}", part));
                        println!(
                            "SENT {} MESSAGE: \"NEWPEER {}\"",
                            message[1].trim(),
                            &network_lock.ip
                        );

                        network_lock.add_and_listen(part.to_string(), peer);
                    }
                }
                "CURRPLAYERS" => println!("  RECIEVED EMPTY CURRPLAYERS COMMAND"),
                command => {
                    panic!("Invalid command {}", command);
                }
            }
        }
        Err(e) => {
            eprintln!("{e}");
        }
    }

    network_lock.add_and_listen(response, peer);
    drop(network_lock);

    let network_clone = network.clone();
    thread::spawn(move || network::start_listening_for_connection(network_clone));

    network
}

fn get_player_ip() -> String {
    let mut response = String::new();

    println!("What is your IP address?");

    io::stdin()
        .read_line(&mut response)
        .expect("Failed to read input from player");

    response.as_str().trim().to_string()
}
