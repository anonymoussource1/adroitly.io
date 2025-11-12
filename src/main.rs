use std::io::{
	self,
	Read,
	Write
};
use std::net::TcpStream;
use std::sync::{
	Arc,
	Mutex
};
use std::thread;
use std::time::Duration;

use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::{
	MouseButton,
	MouseState
};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::EventPump;

mod bullet;
mod helicopter;
mod network;
mod serializer;
mod utils;
mod boundary;

use bullet::Bullet;
use helicopter::Helicopter;
use network::Network;
use serializer::Message;
use utils::{
	get_current_time,
    screenspace_to_worldspace,
    worldspace_to_screenspace,
	Keyboard,
    WORLD_TO_PIXELS
};
use boundary::Boundary;

fn main() -> Result<(), String> {
	let sdl2_context = sdl2::init()?;
	let mut event_pump = match sdl2_context.event_pump() {
		Ok(event_pump) => event_pump,
		Err(err) => {
			panic!("{}", err);
		}
	};
	let video_subsystem = sdl2_context.video()?;
	let window = video_subsystem.window("Adroitly.io", 1500, 1200).position_centered().resizable().build().map_err(|e| e.to_string())?;
	let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
	let mut keyboard = Keyboard::new();
	let network = prompt_for_network();
	let mut shoot_cooldown = Duration::from_secs(0);
	let mut last_time_stamp = Duration::from_secs(0);
	let heli = Arc::new(Mutex::new(Helicopter::new(0.0, 0.0, network.lock().expect("Failed to acquire lock on network").ip.clone())));
    let boundaries = vec![Boundary::new(-54.0, -54.0, 108.0, 4.0), Boundary::new(50.0, -54.0, 4.0, 108.0), Boundary::new(-54.0, -54.0, 4.0, 108.0), Boundary::new(-54.0, 50.0, 108.0, 4.0), Boundary::new(-10.0, -10.0, 20.0, 20.0)];

    {
        let mut network = network.lock().expect("Failed to acquire lock on network");
        let ip = network.ip.clone();
        network.helis.insert(ip, heli.clone());
        network.send_pos(&heli.lock().expect("Failed to acquire lock on heli"));
    }

	canvas.set_draw_color(Color::RGB(20, 20, 20));
	canvas.clear();

	'main: loop {
		let start = get_current_time();
		let mut network = network.lock().expect("Failed to acquire lock on network");
		let delta_time = start - last_time_stamp;

		get_input(&mut event_pump, &mut keyboard);

		let mouse = MouseState::new(&event_pump);

		if keyboard.should_quit {
			break 'main;
		}

		if mouse.is_mouse_button_pressed(MouseButton::Left) && shoot_cooldown == Duration::from_secs(0) {
            let heli = heli.lock().expect("Failed to acquire lock on heli");
            let (mouse_x, mouse_y) = screenspace_to_worldspace((heli.x, heli.y), (mouse.x(), mouse.y()), canvas.window().size());
			let new_x = mouse_x - heli.x - helicopter::SIZE / 2.0;
			let new_y = mouse_y - heli.y - helicopter::SIZE / 2.0;

			let bullet = Bullet::new(
				heli.x + helicopter::SIZE / 2.0,
				heli.y + helicopter::SIZE / 2.0,
				new_x / (new_x.powi(2) + new_y.powi(2)).sqrt(),
				new_y / (new_x.powi(2) + new_y.powi(2)).sqrt()
			);

			network.send_bullet(&bullet);

			let ip = network.ip.clone();
			if let Some(specific_bullets) = network.bullets.get_mut(&ip) {
				specific_bullets.lock().expect("Failed to acquire lock on specific_bullets").push(bullet);
			} else {
				let specific_bullets = Arc::new(Mutex::new(Vec::new()));

				specific_bullets.lock().expect("Failed to acquire lock on specific_bullets").push(bullet);

				network.bullets.insert(ip, specific_bullets);
			}

			shoot_cooldown = Duration::from_millis(250);
		}

		// END OF INPUT

		if delta_time <= shoot_cooldown {
			shoot_cooldown -= delta_time;
		} else {
			shoot_cooldown = Duration::from_secs(0);
		}

		for bullets in network.bullets.values() {
			for bullet in bullets.lock().expect("Failed to acquire lock on bullets").iter_mut() {
				bullet.update(&delta_time);
			}
		}

        {
            let mut heli = heli.lock().expect("Failed to acquire lock on heli");
            let old_pos = (heli.x, heli.y);
            heli.update(&delta_time, &keyboard, &boundaries);

		// END OF PHYSICS

            if old_pos != (heli.x, heli.y) {
                network.send_pos(&heli);
            }
        }

		// END OF NETWORK

		canvas.set_draw_color(Color::RGB(20, 20, 20));
		canvas.clear();

        let focus = {
            let heli = network.helis.get(&network.ip).unwrap().lock().expect("Failed to acquire lock on helicoper");
            (heli.x, heli.y)
        };

        canvas.set_draw_color(Color::RGB(45, 45, 45));
        for r in (-50..=50).step_by(5) {
            for c in (-50..=50).step_by(5) {
                let (x, y) = worldspace_to_screenspace(focus, (r as f64 - 0.25, c as f64 - 0.25), canvas.window().size());
                canvas.fill_rect(Rect::new(x, y, (0.5 * WORLD_TO_PIXELS) as u32, (0.5 * WORLD_TO_PIXELS) as u32))?;
            }
        }

		for (ip, curr_heli) in network.helis.iter() {
			if ip == &network.ip {
				canvas.set_draw_color(Color::RGB(225, 100, 100));
			} else {
				canvas.set_draw_color(Color::RGB(100, 100, 225));
            }

			curr_heli.lock().expect("Failed to acquire lock on helicopter").draw(focus, &mut canvas)?;
		}

		for (ip, bullets) in network.bullets.iter() {
			if ip == &network.ip {
				canvas.set_draw_color(Color::RGB(225, 100, 100));
			} else {
				canvas.set_draw_color(Color::RGB(100, 100, 225));
			}

			for bullet in bullets.lock().expect("Failed to acquire lock on bullets").iter() {
				bullet.draw(focus, &mut canvas)?;
			}
		}

        for boundary in boundaries.iter() {
            boundary.draw(focus, &mut canvas)?;
        }

		canvas.present();

		drop(network);

		last_time_stamp = get_current_time();
		if last_time_stamp - start <= Duration::from_millis(1000 / 60) {
			thread::sleep(Duration::from_millis(1000 / 60) - (last_time_stamp - start));
		}
	}

	Ok(())
}

fn get_input(event_pump: &mut EventPump, keyboard: &mut Keyboard) {
	for event in event_pump.poll_iter() {
		match event {
			Event::Quit {
				..
			}
			| Event::KeyDown {
				keycode: Some(Keycode::Escape), ..
			} => keyboard.should_quit = true,
			Event::KeyDown {
				keycode: Some(Keycode::W), ..
			} => keyboard.is_w_down = true,
			Event::KeyDown {
				keycode: Some(Keycode::A), ..
			} => keyboard.is_a_down = true,
			Event::KeyDown {
				keycode: Some(Keycode::S), ..
			} => keyboard.is_s_down = true,
			Event::KeyDown {
				keycode: Some(Keycode::D), ..
			} => keyboard.is_d_down = true,
			Event::KeyUp {
				keycode: Some(Keycode::W), ..
			} => keyboard.is_w_down = false,
			Event::KeyUp {
				keycode: Some(Keycode::A), ..
			} => keyboard.is_a_down = false,
			Event::KeyUp {
				keycode: Some(Keycode::S), ..
			} => keyboard.is_s_down = false,
			Event::KeyUp {
				keycode: Some(Keycode::D), ..
			} => keyboard.is_d_down = false,
			_ => ()
		}
	}
}

fn prompt_for_network() -> Arc<Mutex<Network>> {
	let mut response = String::new();

	println!("Welcome to Adroitly.io! Would you like to...");
	println!("  [1] Create a game");
	println!("  [2] Join a friend");

	io::stdin().read_line(&mut response).expect("Failed to read input from player");

	match response.as_str().trim() {
		"1" => create_game(),
		"2" => connect_to_game(),
		_ => panic!("Incorrect response!")
	}
}

fn create_game() -> Arc<Mutex<Network>> {
	let ip = get_player_ip();
	let network = Arc::new(Mutex::new(Network::new(&ip)));

	let network_clone = network.clone();
	thread::spawn(move || network::start_listening_for_connection(network_clone));

	network
}

fn connect_to_game() -> Arc<Mutex<Network>> {
	let mut response = String::new();

	println!("What is one of the player's IP address?");

	io::stdin().read_line(&mut response).expect("Failed to read input from player");

	response = response.as_str().trim().to_string();

	let mut peer = TcpStream::connect(&response).expect(&format!("Failed to connect to player at IP address \"{}\"", &response));

	let player_ip = get_player_ip();
	let network = Arc::new(Mutex::new(Network::new(&player_ip)));

	let join = Message::Join(true, player_ip.clone());

	peer.write_all(&join.serialize()).expect("Failed to write to player");
	println!("Sent {} join request...", response);

    let mut buffer = [0; 1024];
    match peer.read(&mut buffer) {
        Ok(0) => panic!("ThIS sHOuLd NoT bE HaPPeNIng"),
        Ok(bytes_read) => {
            let message = Message::deserialize(&buffer[..bytes_read]);

            match message {
                Message::CurrPlayers(ips) => {
                    println!("Recieved current players...");
                    let mut network_lock = network.lock().expect("Failed to acquire lock on network");

                    for ip in ips {
                        if ip == player_ip { continue; }

                        if let Ok(mut peer) = TcpStream::connect(&ip) {
                            let join = Message::Join(false, network_lock.ip.clone());
                            peer.write_all(&join.serialize()).expect(&format!("Failed to write to IP address {}", &ip));

                            let ip_thread = network_lock.add_and_listen(ip, peer);
                            let network = network.clone();
                            thread::spawn(move || {
                                let ip = ip_thread.join().expect("Something, somewhere, went wrong");
                                let mut network = network.lock().expect("Failed to acquire lock on network");
                                network.peers.remove(&ip);
                                network.helis.remove(&ip);
                                network.bullets.remove(&ip);
                            });
                        }
                    }

                    let ip_thread = network_lock.add_and_listen(response.clone(), peer);
                    let network = network.clone();
                    thread::spawn(move || {
                        let ip = ip_thread.join().expect("Something, somewhere, went wrong");
                        let mut network = network.lock().expect("Failed to acquire lock on network");
                        network.peers.remove(&ip);
                        network.helis.remove(&ip);
                        network.bullets.remove(&ip);
                    });
                }
                _ => unreachable!()
            }

            let network = network.clone();
            thread::spawn(move || network::start_listening_for_connection(network));
        }
        Err(e) => eprintln!("{e}"),
    }

	network
}

fn get_player_ip() -> String {
	let mut response = String::new();

	println!("What is your IP address?");

	io::stdin().read_line(&mut response).expect("Failed to read input from player");

	response.as_str().trim().to_string()
}
