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
use sdl2::EventPump;

mod bullet;
mod helicopter;
mod network;
mod serializer;
mod utils;

use bullet::Bullet;
use helicopter::Helicopter;
use network::Network;
use serializer::Message;
use utils::{
	get_current_time,
	Keyboard
};

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
	let mut heli = Helicopter::new(4, 6, network.lock().expect("Failed to acquire lock on network").ip.clone());

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
			let new_x = mouse.x() as f64 - heli.x - helicopter::SIZE as f64 / 2.0;
			let new_y = mouse.y() as f64 - heli.y - helicopter::SIZE as f64 / 2.0;

			let bullet = Bullet::new(
				heli.x + helicopter::SIZE as f64 / 2.0,
				heli.y + helicopter::SIZE as f64 / 2.0,
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

        let old_pos = (heli.x, heli.y);
		heli.update(&delta_time, &keyboard);

		for bullets in network.bullets.values() {
			for bullet in bullets.lock().expect("Failed to acquire lock on bullets").iter_mut() {
				bullet.update(&delta_time);
			}
		}

		// END OF PHYSICS

        if old_pos != (heli.x, heli.y) {
            network.send_pos(&heli);
        }

		// END OF NETWORK

		canvas.set_draw_color(Color::RGB(20, 20, 20));
		canvas.clear();

		canvas.set_draw_color(Color::RGB(225, 100, 100));
		heli.draw(&mut canvas)?;

		for heli in network.helis.values() {
			canvas.set_draw_color(Color::RGB(100, 100, 225));
			heli.lock().expect("Failed to acquire lock on helicopter").draw(&mut canvas)?;
		}

		for (ip, bullets) in network.bullets.iter() {
			if ip == &network.ip {
				canvas.set_draw_color(Color::RGB(225, 100, 100));
			} else {
				canvas.set_draw_color(Color::RGB(100, 100, 225));
			}

			for bullet in bullets.lock().expect("Failed to acquire lock on bullets").iter() {
				bullet.draw(&mut canvas)?;
			}
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

	let ip = get_player_ip();
	let network = Arc::new(Mutex::new(Network::new(&ip)));

	let join = Message::Join(true, ip);

	peer.write_all(&join.serialize()).expect("Failed to write to player");
	println!("SENT {} \"{}\"", response, join);

	let mut buffer = [0; 1024];
	match peer.read(&mut buffer) {
		Ok(0) => {
			panic!("ThIS sHOuLd NoT bE HaPPeNIng");
		}
		Ok(bytes_read) => {
			let message = Message::deserialize(&buffer[..bytes_read]);

			println!("  RECIEVED \"{}\"", message);
			match message {
				Message::GameState(ips, bullets) => {
					let mut network = network.lock().expect("Failed to acquire lock on network");

					for ip in ips {
						let mut peer = TcpStream::connect(&ip).expect(&format!("Failed to connect to IP address {}", &ip));
						let join = Message::Join(false, network.ip.clone());
						peer.write_all(&join.serialize()).expect(&format!("Failed to write to IP address {}", &ip));
						println!("  SENT {} \"{}\"", ip, join);

						network.add_and_listen(ip, peer);
					}



					network.add_and_listen(response, peer);
				}
				_ => unreachable!()
			}

			let network = network.clone();
			thread::spawn(move || network::start_listening_for_connection(network));
		}
		Err(e) => {
			eprintln!("{e}");
		}
	}

	network
}

fn get_player_ip() -> String {
	let mut response = String::new();

	println!("What is your IP address?");

	io::stdin().read_line(&mut response).expect("Failed to read input from player");

	response.as_str().trim().to_string()
}
