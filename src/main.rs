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
use std::time::{
	Duration,
	SystemTime,
	UNIX_EPOCH
};

use sdl2;
use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::{
	MouseButton,
	MouseState
};
use sdl2::pixels::{
	Color,
	PixelFormatEnum
};
use sdl2::rect::{
	Point,
	Rect
};
use sdl2::render::BlendMode;
use sdl2::surface::Surface;

mod boundary;
mod bullet;
mod camera;
mod fort;
mod helicopter;
mod keyboard;
mod network;
mod serializer;

use boundary::Boundary;
use bullet::Bullet;
use camera::{
	WORLD_TO_PIXELS,
	screenspace_to_worldspace,
	worldspace_to_screenspace
};
use fort::Fort;
use helicopter::Helicopter;
use keyboard::Keyboard;
use network::Network;
use serializer::Message;

fn main() -> Result<(), String> {
	let sdl2 = sdl2::init()?;
	let mut events = match sdl2.event_pump() {
		Ok(events) => events,
		Err(err) => {
			panic!("{}", err);
		}
	};
	let video = sdl2.video()?;
	let window = video.window("Adroitly.io", 1500, 1200).position_centered().resizable().build().map_err(|e| e.to_string())?;
	let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
	let mut keyboard = Keyboard::new();

	let network = prompt_for_network();

	let boundaries = vec![
		Boundary::new(-54.0, -54.0, 108.0, 4.0),
		Boundary::new(50.0, -54.0, 4.0, 108.0),
		Boundary::new(-54.0, -54.0, 4.0, 108.0),
		Boundary::new(-54.0, 50.0, 108.0, 4.0),
		Boundary::new(-10.0, -10.0, 20.0, 20.0),
	];
	let spawn = helicopter::find_valid_spawn(&boundaries);
	let heli_mutex = Arc::new(Mutex::new(Helicopter::new(spawn.0, spawn.1)));

	{
		let mut network = network.lock().expect("Failed to acquire lock on network");
		let ip = network.ip.clone();
		network.helis.insert(ip, heli_mutex.clone());
		network.send_pos(&heli_mutex.lock().expect("Failed to acquire lock on heli"));
	}

	canvas.set_draw_color(Color::RGB(20, 20, 20));
	canvas.clear();

	let mut shoot_cooldown = Duration::ZERO;
	let mut death_timer = Duration::ZERO;
	let mut last_start = get_current_time();
	let mut curr_fort_index = 0;

	'main: loop {
		let start = get_current_time();
		let mut network = network.lock().expect("Failed to acquire lock on network");
		let mut heli = heli_mutex.lock().expect("Failed to acquire lock on heli");
		let delta_time = start - last_start;

		get_input(&mut events, &mut keyboard);

		let mouse = MouseState::new(&events);

		if keyboard.should_quit {
			break 'main;
		}

		if mouse.is_mouse_button_pressed(MouseButton::Left) && shoot_cooldown == Duration::from_secs(0) && !heli.is_dead {
			let (mouse_x, mouse_y) = screenspace_to_worldspace((heli.x, heli.y), (mouse.x(), mouse.y()), canvas.window().size());
			let new_x = mouse_x - heli.x - helicopter::SIZE / 2.0;
			let new_y = mouse_y - heli.y - helicopter::SIZE / 2.0;

			let bullet = Bullet::new(
				heli.x + helicopter::SIZE / 2.0,
				heli.y + helicopter::SIZE / 2.0,
				new_x / (new_x * new_x + new_y * new_y).sqrt(),
				new_y / (new_x * new_x + new_y * new_y).sqrt(),
				0
			);

			network.send_bullet(&bullet);

			let ip = network.ip.clone();
			network.bullets.get_mut(&ip).unwrap().lock().expect("Failed to acquire lock on bullets").push(bullet);

			shoot_cooldown = Duration::from_millis(250);
		}

		if keyboard.is_space_down && !heli.is_dead {
			let margin = (helicopter::SIZE - fort::SIZE) / 2.0;
			let fort = Fort::new(heli.x + margin, heli.y + margin);
			let ip = network.ip.clone();
			let mut forts = network.forts.get(&ip).unwrap().lock().expect("Failed to acquire lock on forts");

			if let Some((index, other)) = forts.iter().enumerate().find(|&other| {
				let other = other.1;
				let distance = ((fort.y - other.y) * (fort.y - other.y) + (fort.x - other.x) * (fort.x - other.x)).sqrt().abs();

				distance < 3.0
			}) {
				let curr_fort = forts.get(curr_fort_index).unwrap();
				let is_valid_connection = !curr_fort.connections.contains(&(other.x, other.y)) && !other.connections.contains(&(curr_fort.x, curr_fort.y));
				let other = (other.x, other.y);
				let curr_fort = forts.get_mut(curr_fort_index).unwrap();

				if other.0 != curr_fort.x || other.1 != curr_fort.y {
					curr_fort_index = index;

					let distance = ((curr_fort.y - other.1) * (curr_fort.y - other.1) + (curr_fort.x - other.0) * (curr_fort.x - other.0)).sqrt().abs();
					if distance < 15.0 && is_valid_connection {
						curr_fort.connections.push((other.0, other.1));
						let curr = (curr_fort.x, curr_fort.y);
						drop(forts);
						network.send_fort_connection(((curr.0, curr.1), (other.0, other.1)));
					}
				}
			} else if !forts.is_empty() {
				let curr_fort = forts.get(curr_fort_index).unwrap();
				let distance = ((heli.y - curr_fort.y) * (heli.y - curr_fort.y) + (heli.x - curr_fort.x) * (heli.x - curr_fort.x)).sqrt().abs();

				if distance > 3.0 {
					drop(forts);
					network.send_fort(&fort);
					let mut forts = network.forts.get_mut(&ip).unwrap().lock().expect("Failed to acquire lock on forts");
					let curr_fort = forts.get_mut(curr_fort_index).unwrap();

					if distance < 15.0 {
						curr_fort.connections.push((fort.x, fort.y));
						let curr = (curr_fort.x, curr_fort.y);
						let other = (fort.x, fort.y);
						forts.push(fort);
						curr_fort_index = forts.len() - 1;
						drop(forts);
						network.send_fort_connection(((curr.0, curr.1), (other.0, other.1)));
					} else {
						forts.push(fort);
						curr_fort_index = forts.len() - 1;
					}
				}
			} else {
				drop(forts);
				network.send_fort(&fort);
				network.forts.get(&network.ip).unwrap().lock().expect("Failed to acquire lock on forts").push(fort);
			}

			keyboard.is_space_down = false;
		}

		// END OF INPUT

		if delta_time <= shoot_cooldown {
			shoot_cooldown -= delta_time;
		} else {
			shoot_cooldown = Duration::ZERO;
		}

		// Can this...
		for bullets in network.bullets.values_mut() {
			let mut bullets = bullets.lock().expect("Failed to acquire lock on bullets");
			for bullet in bullets.iter_mut() {
				bullet.update(&delta_time);
			}
			bullets.retain(|bullet| bullet.age < bullet::LIFESPAN);
		}

		let old_pos = (heli.x, heli.y);
		if !heli.is_dead {
			heli.update(&delta_time, &keyboard, &boundaries);

			for (ip, forts) in network.forts.iter() {
				if ip == &network.ip {
					continue;
				}
				for fort in forts.lock().expect("Failed to acquire lock on forts").iter() {
					for connection in fort.connections.iter() {
						let connection = ((fort.x, fort.y), (connection.0, connection.1));
						let primary = ((heli.x, heli.y), (heli.x + helicopter::SIZE, heli.y));
						let secondary = ((heli.x + helicopter::SIZE, heli.y), (heli.x, heli.y + helicopter::SIZE));
						let tertiary = ((heli.x + helicopter::SIZE, heli.y + helicopter::SIZE), (heli.x, heli.y + helicopter::SIZE));
						let quaternary = ((heli.x, heli.y + helicopter::SIZE), (heli.x, heli.y));

						if do_segments_intersect_with_thickness(connection, primary, 0.5)
							|| do_segments_intersect_with_thickness(connection, secondary, 0.5)
							|| do_segments_intersect_with_thickness(connection, tertiary, 0.5)
							|| do_segments_intersect_with_thickness(connection, quaternary, 0.5)
						{
							let spawn = helicopter::find_valid_spawn(&boundaries);

							heli.x = spawn.0;
							heli.y = spawn.1;
							death_timer = Duration::from_secs(5);
						}
					}
				}
			}
		}

		let mut removed_forts = Vec::new();
		for (ip, bullets) in network.bullets.iter() {
			for bullet in bullets.lock().expect("Failed to acquire lock on bullets").iter_mut() {
				let x = bullet.x - bullet::DIAMETER / 2.0;
				let y = bullet.y - bullet::DIAMETER / 2.0;

				if ip != &network.ip {
					// ..And this be combined?
					if !heli.is_dead && x < heli.x + helicopter::SIZE && x + bullet::DIAMETER > heli.x && y < heli.y + helicopter::SIZE && y + bullet::DIAMETER > heli.y {
						let spawn = helicopter::find_valid_spawn(&boundaries);

						heli.x = spawn.0;
						heli.y = spawn.1;
						death_timer = Duration::from_secs(5);
					}

					let mut forts = network.forts.get(&network.ip).unwrap().lock().expect("Failed to acquire lock on forts");
					let prev_len = forts.len();

					forts.retain(|fort| {
						if x < fort.x + helicopter::SIZE && x + bullet::DIAMETER > fort.x && y < fort.y + helicopter::SIZE && y + bullet::DIAMETER > fort.y {
							removed_forts.push((fort.x, fort.y));
							false
						} else {
							true
						}
					});
					forts.iter_mut().for_each(|fort| fort.connections.retain(|connection| !removed_forts.contains(connection)));

					if curr_fort_index != 0 {
						curr_fort_index = curr_fort_index - (prev_len - forts.len());
					}
				}

				for (fort_ip, forts) in network.forts.iter() {
					if ip == fort_ip {
						continue;
					}

					for fort in forts.lock().expect("Failed to acquire lock on fort").iter() {
						for connection in fort.connections.iter() {
							let connection = ((fort.x, fort.y), (connection.0, connection.1));
							let primary = ((x, y), (x + bullet::DIAMETER, y));
							let secondary = ((x + bullet::DIAMETER, y), (x, y + bullet::DIAMETER));
							let tertiary = ((x + bullet::DIAMETER, y + bullet::DIAMETER), (x, y + bullet::DIAMETER));
							let quaternary = ((x, y + bullet::DIAMETER), (x, y));
							let normals = get_normals(connection.0, connection.1);

							if do_segments_intersect_with_thickness(connection, primary, 0.5)
								|| do_segments_intersect_with_thickness(connection, secondary, 0.5)
								|| do_segments_intersect_with_thickness(connection, tertiary, 0.5)
								|| do_segments_intersect_with_thickness(connection, quaternary, 0.5) {

								let temp_angle = (normals.0.1 / normals.0.0).atan();
								let angle = if normals.0.0 < 0.0 { temp_angle + std::f64::consts::PI } else { temp_angle };
								let rotated_x = bullet.dx * (-angle).cos() - bullet.dy * (-angle).sin();
								let rotated_y = bullet.dx * (-angle).sin() + bullet.dy * (-angle).cos();
								let reflected_x = -rotated_x;
								let reflected_y = rotated_y;
								let final_x = reflected_x * angle.cos() - reflected_y * angle.sin();
								let final_y = reflected_x * angle.sin() + reflected_y * angle.cos();

								bullet.dx = final_x;
								bullet.dy = final_y;
							}
						}
					}
					
				}
			}
		}
		removed_forts.iter().for_each(|fort| network.send_fort(&Fort::new(fort.0, fort.1)));

		// END OF PHYSICS

		if old_pos != (heli.x, heli.y) {
			network.send_pos(&heli);
		}

		if death_timer == Duration::from_secs(5) {
			heli.is_dead = true;
			network.send_death();
		} else if heli.is_dead && death_timer.is_zero() {
			heli.is_dead = false;
			network.send_death();
		}

		if delta_time <= death_timer {
			death_timer -= delta_time;
		} else {
			death_timer = Duration::ZERO;
		}

		// END OF NETWORK

		canvas.set_draw_color(Color::RGB(20, 20, 20));
		canvas.clear();

		let focus = (heli.x + helicopter::SIZE / 2.0, heli.y + helicopter::SIZE / 2.0);

		canvas.set_draw_color(Color::RGB(45, 45, 45));
		for r in (-50..=50).step_by(5) {
			for c in (-50..=50).step_by(5) {
				let (x, y) = worldspace_to_screenspace(focus, (r as f64 - 0.25, c as f64 - 0.25), canvas.window().size());
				canvas.fill_rect(Rect::new(x, y, (0.5 * WORLD_TO_PIXELS) as u32, (0.5 * WORLD_TO_PIXELS) as u32))?;
			}
		}

		drop(heli);

		if keyboard.is_p_down {
			println!("--DEBUG INFO--");
			for (ip, bullets) in network.bullets.iter() {
				println!("BULLETS ON {}: {:?}", ip, bullets.lock().expect("Failed to acquire lock on bullets"));
			}
			for (ip, heli) in network.helis.iter() {
				println!("HELI ON {}: {:?}", ip, heli.lock().expect("Failed to acquire lock on heli"));
			}
			println!("DEATH TIMER: {:?}", death_timer);
			keyboard.is_p_down = false;
		}

		for (ip, heli) in network.helis.iter() {
			let heli = heli.lock().expect("Failed to acquire lock on helicopter");
			if ip == &network.ip {
				canvas.set_draw_color(Color::RGB(225, 100, 100));
			} else {
				canvas.set_draw_color(Color::RGB(100, 100, 225));
			}

			if heli.is_dead {
				let (x, y) = worldspace_to_screenspace(focus, (heli.x, heli.y), canvas.window().size());
				let size = (helicopter::SIZE * WORLD_TO_PIXELS) as u32;
				canvas.draw_rect(Rect::new(x, y, size, size))?;
			} else {
				heli.draw(focus, &mut canvas)?;
			}
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

		for (ip, forts) in network.forts.iter() {
			let color = if ip == &network.ip {
				canvas.set_draw_color(Color::RGB(225, 100, 100));
				Color::RGB(225, 100, 100)
			} else {
				canvas.set_draw_color(Color::RGB(100, 100, 225));
				Color::RGB(100, 100, 225)
			};

			for fort in forts.lock().expect("Failed to acquire lock on forts").iter() {
				fort.draw_line(focus, &mut canvas, color)?;
			}

			for fort in forts.lock().expect("Failed to acquire lock on forts").iter() {
				fort.draw(focus, &mut canvas)?;
			}
		}

		let heli = heli_mutex.lock().expect("Failed to acquire lock on heli");

		if let Some(fort) = network.forts.get(&network.ip).unwrap().lock().expect("Failed to acquire lock on fort").get(curr_fort_index) {
			let (x, y) = worldspace_to_screenspace(focus, (fort.x, fort.y), canvas.window().size());

			let distance = ((heli.y - fort.y) * (heli.y - fort.y) + (heli.x - fort.x) * (heli.x - fort.x)).sqrt().abs();
			if distance as u32 != 0 && distance < 15.0 {
				let temp_angle = ((heli.x - fort.x) / distance).acos() * (180.0 / std::f64::consts::PI);
				let angle = if (heli.y - fort.y) < 0.0 { -temp_angle } else { temp_angle };

				let texture_creator = canvas.texture_creator();
				let mut surface = Surface::new((distance * WORLD_TO_PIXELS) as u32, (fort::CONNECTION_HEIGHT * WORLD_TO_PIXELS) as u32, PixelFormatEnum::RGB24)?;

				surface.fill_rect(
					Rect::new(0, 0, (distance * WORLD_TO_PIXELS) as u32, (fort::CONNECTION_HEIGHT * WORLD_TO_PIXELS) as u32),
					Color::RGB(225, 100, 100)
				)?;

				let mut texture = surface.as_texture(&texture_creator).unwrap();
				texture.set_blend_mode(BlendMode::Blend);
				texture.set_alpha_mod(125);

				canvas.copy_ex(
					&texture,
					None,
					Some(Rect::new(
						x + (fort::SIZE / 2.0 * WORLD_TO_PIXELS) as i32,
						y + ((fort::SIZE / 2.0 - 0.25) * WORLD_TO_PIXELS) as i32,
						(distance * WORLD_TO_PIXELS) as u32,
						(fort::CONNECTION_HEIGHT * WORLD_TO_PIXELS) as u32
					)),
					angle,
					Some(Point::new(0, (0.25 * WORLD_TO_PIXELS) as i32)),
					false,
					false
				)?;
			}
		}

		for boundary in boundaries.iter() {
			boundary.draw(focus, &mut canvas)?;
		}

		canvas.present();

		drop(network);

		let time_elapsed_so_far = get_current_time();
		if time_elapsed_so_far - start <= Duration::from_millis((1000.0 / 60.0) as u64) {
			thread::sleep(Duration::from_millis((1000.0 / 60.0) as u64) - (time_elapsed_so_far - start));
		}
		last_start = start;
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
			Event::KeyDown {
				keycode: Some(Keycode::Space), ..
			} => keyboard.is_space_down = true,
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
			Event::KeyUp {
				keycode: Some(Keycode::Space), ..
			} => keyboard.is_space_down = false,
			Event::KeyDown {
				keycode: Some(Keycode::P), ..
			} => keyboard.is_p_down = true,
			_ => ()
		}
	}
}

fn prompt_for_network() -> Arc<Mutex<Network>> {
	let mut response = String::new();

	println!("Welcome to Adroitly.io, the premier helicopter gaming experience! Would you like to...");
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
	//let ip = get_player_ip();
	let ip = String::from("10.0.0.65:8080");
	let network = Arc::new(Mutex::new(Network::new(&ip)));

	let network_clone = network.clone();
	thread::spawn(move || network::start_listening_for_connection(network_clone));

	network
}

fn connect_to_game() -> Arc<Mutex<Network>> {
	let /*mut*/ response = String::from("10.0.0.65:8080");

	//println!("What is one of the player's IP address?");

	//io::stdin().read_line(&mut response).expect("Failed to read input from
	// player");

	//response = response.as_str().trim().to_string();

	let mut player = TcpStream::connect(&response).expect(&format!("Failed to connect to player at IP address \"{}\"", &response));

	//let player_ip = get_player_ip();
	let player_ip = String::from("10.0.0.65:8081");
	let network = Arc::new(Mutex::new(Network::new(&player_ip)));

	let join = Message::Join(true, player_ip.clone());

	player.write_all(&join.serialize()).expect("Failed to write to player");
	println!("Sent {} join request...", response);

	let mut buffer = [0; 1024];
	match player.read(&mut buffer) {
		Ok(0) => panic!("ThIS sHOuLd NoT bE HaPPeNIng"),
		Ok(bytes_read) => {
			let Some(message) = Message::deserialize(&buffer[..bytes_read]) else {
				panic!("Did not recieve valid message during initialization")
			};

			match message {
				Message::CurrPlayers(ips) => {
					println!("  Recieved current players...");
					let mut network_lock = network.lock().expect("Failed to acquire lock on network");

					for ip in ips {
						if ip == player_ip {
							continue;
						}

						if let Ok(mut player) = TcpStream::connect(&ip) {
							let join = Message::Join(false, network_lock.ip.clone());
							player.write_all(&join.serialize()).expect(&format!("Failed to write to IP address {}", &ip));

							let ip_thread = network_lock.add_and_listen(ip, player);
							let network = network.clone();
							thread::spawn(move || {
								let ip = ip_thread.join().expect("Something, somewhere, went wrong");
								let mut network = network.lock().expect("Failed to acquire lock on network");
								network.players.remove(&ip);
								network.helis.remove(&ip);
								network.bullets.remove(&ip);
							});
						}
					}

					let ip_thread = network_lock.add_and_listen(response.clone(), player);
					let network = network.clone();
					thread::spawn(move || {
						let ip = ip_thread.join().expect("Something, somewhere, went wrong");
						let mut network = network.lock().expect("Failed to acquire lock on network");
						network.players.remove(&ip);
						network.helis.remove(&ip);
						network.bullets.remove(&ip);
					});
				}
				_ => panic!("Did not recieve CurrPlayers during initialization")
			}

			let network = network.clone();
			thread::spawn(move || network::start_listening_for_connection(network));
		}
		Err(e) => eprintln!("{e}")
	}

	network
}

fn get_player_ip() -> String {
	let mut response = String::new();

	println!("What is your IP address?");

	io::stdin().read_line(&mut response).expect("Failed to read input from player");

	response.as_str().trim().to_string()
}

fn get_current_time() -> Duration {
	SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards and we're all doomed anyway")
}

// Black magic I found on the internets
fn is_counter_clockwise(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> bool {
	(c.1 - a.1) * (b.0 - a.0) > (b.1 - a.1) * (c.0 - a.0)
}

fn do_segments_intersect(a: ((f64, f64), (f64, f64)), b: ((f64, f64), (f64, f64))) -> bool {
	is_counter_clockwise(a.0, b.0, b.1) != is_counter_clockwise(a.1, b.0, b.1) && is_counter_clockwise(a.0, a.1, b.0) != is_counter_clockwise(a.0, a.1, b.1)
}

fn do_segments_intersect_with_thickness(a: ((f64, f64), (f64, f64)), b: ((f64, f64), (f64, f64)), thickness: f64) -> bool {
	let thick_a = ((a.0.0, a.0.1 + thickness), (a.1.0, a.1.1 + thickness));
	do_segments_intersect(a, b) || do_segments_intersect(thick_a, b)
}

fn get_normals(a: (f64, f64), b: (f64, f64)) -> ((f64, f64), (f64, f64)) {
	// if dx == 0 then dy = 1
	// if dy == 0 then dx = 1
	// sqrt(dx^2 + dy^2) = 1
	let dx = b.0 - a.0;
	let dy = b.1 - a.1;
	let length = (dx * dx + dy * dy).sqrt();
	((-dy / length, dx / length), (dy / length, -dx / length))
}
