use std::sync::{
	Arc,
	Mutex
};
use std::time::{
	Duration,
	SystemTime,
	UNIX_EPOCH
};
use std::{
	io,
	thread
};

use sdl2;
use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::{
	MouseButton,
	MouseState
};
use sdl2::pixels::Color;
use sdl2::rect::Rect;

mod boundary;
mod bullet;
mod camera;
mod fort;
mod helicopter;
mod keyboard;
mod network;
mod serializer;
mod shapes;

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
use shapes::*;

enum VerticalDirection {
	Up,
	Down,
}

enum HorizontalDirection {
	Left,
	Right,
}

enum GameEvent {
	NewHeliDirection(Option<VerticalDirection>, Option<HorizontalDirection>),
	PlayerDeath,
	BulletCreation(Bullet),
	BulletDestruction(String, usize),
	BulletReflection(String, usize, Segment),
	FortCreation(Fort),
	FortHit(String, usize),
	FortConnectionCreation(usize, Vec2),
	CurrFortChanged(usize),
}

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
		let mut game_events = Vec::new();

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

			game_events.push(GameEvent::BulletCreation(bullet));
		}

		let vertical = if keyboard.is_w_down {
			Some(VerticalDirection::Up)
		} else if keyboard.is_s_down {
			Some(VerticalDirection::Down)
		} else {
			None
		};

		let horizontal = if keyboard.is_a_down {
			Some(HorizontalDirection::Left)
		} else if keyboard.is_d_down {
			Some(HorizontalDirection::Right)
		} else {
			None
		};

		game_events.push(GameEvent::NewHeliDirection(vertical, horizontal));

		if !heli.is_dead && keyboard.is_space_down {
			let margin = (helicopter::SIZE - fort::SIZE) / 2.0;
			let forts = network.forts.get(&network.ip).unwrap().lock().expect("Failed to acquire lock on forts");
			let new_fort = Fort::new(heli.x + margin, heli.y + margin);

			let mut min_distance = f64::MAX;
			let mut closest_fort_index = 0;
			for (index, fort) in forts.iter().enumerate() {
				let distance = (new_fort.y - fort.y) * (new_fort.y - fort.y) + (new_fort.x - fort.x) * (new_fort.x - fort.x);
				if distance < min_distance { 
					min_distance = distance;
					closest_fort_index = index;
				}
			}

			if forts.is_empty() {
				game_events.push(GameEvent::FortCreation(new_fort));
			} else {
				let curr_fort = forts.get(curr_fort_index).unwrap();
				let closest_fort = forts.get(closest_fort_index).unwrap();

				let curr_fort_distance = (new_fort.y - curr_fort.y) * (new_fort.y - curr_fort.y) + (new_fort.x - curr_fort.x) * (new_fort.x - curr_fort.x);
				let closest_fort_distance = (new_fort.y - closest_fort.y) * (new_fort.y - closest_fort.y) + (new_fort.x - closest_fort.x) * (new_fort.x - closest_fort.x);

				match (curr_fort_distance > 15.0 * 15.0, closest_fort_distance < 3.0 * 3.0) {
					(false, false) => {
						game_events.push(GameEvent::FortConnectionCreation(curr_fort_index, Vec2::new(new_fort.x, new_fort.y)));
						game_events.push(GameEvent::FortCreation(new_fort));
					},
					(true, false) => game_events.push(GameEvent::FortCreation(new_fort)),
					(false, true) => if curr_fort_index != closest_fort_index {
						if !closest_fort.connections.contains(&(curr_fort.x, curr_fort.y)) && !curr_fort.connections.contains(&(closest_fort.x, closest_fort.y)){
							game_events.push(GameEvent::FortConnectionCreation(curr_fort_index, Vec2::new(closest_fort.x, closest_fort.y)));
						}
						game_events.push(GameEvent::CurrFortChanged(closest_fort_index));
					},
					(true, true) => game_events.push(GameEvent::CurrFortChanged(closest_fort_index)),
				}
			}
		}

		for (bullet_ip, bullets) in network.bullets.iter() {
			for (bullet_index, bullet) in bullets.lock().expect("Failed to acquire lock on bullets").iter().enumerate() {
				// TODO, Potential issue, if sending bullet lags a lot, lifetime & physics would be
				// behind which is desync. Maybe instead of simulating, we could send with a
				// timestamp? Need to research
				if bullet.age >= bullet::LIFESPAN {
					game_events.push(GameEvent::BulletDestruction(bullet_ip.clone(), bullet_index));
				}

				if bullet_ip != &network.ip && !heli.is_dead && heli.bounds().intersects_with_rect(&bullet.bounds()) {
					game_events.push(GameEvent::PlayerDeath);
					game_events.push(GameEvent::BulletDestruction(bullet_ip.clone(), bullet_index));

					continue;
				}

				for (fort_ip, forts) in network.forts.iter() {
					for (fort_index, fort) in forts.lock().expect("Failed to acquire lock on forts").iter().enumerate() {
						if fort_ip == bullet_ip { continue; }

						if fort.bounds().intersects_with_rect(&bullet.bounds()) {
							game_events.push(GameEvent::FortHit(fort_ip.clone(), fort_index));
							game_events.push(GameEvent::BulletDestruction(bullet_ip.clone(), bullet_index));

							continue;
						}

						for connection in fort.connections.iter() {
							let wall_segment = Segment::new(Vec2::new(fort.x + fort::SIZE / 2.0, fort.y + fort::SIZE / 2.0), Vec2::new(connection.0 + fort::SIZE / 2.0, connection.1 + fort::SIZE / 2.0), fort::CONNECTION_HEIGHT);
							if bullet.bounds().intersects_with_segment(&wall_segment) {
								game_events.push(GameEvent::BulletReflection(bullet_ip.clone(), bullet_index, wall_segment));
							}
						}
					}
				}
			}
		}

		for event in game_events.into_iter() {
			match event {
				GameEvent::NewHeliDirection(vertical, horizontal) => {
					let delta_time = delta_time.as_millis() as f64 / 1000.0;
					let mut delta_y = 0.0;
					let mut delta_x = 0.0;

					match vertical {
						Some(VerticalDirection::Up) => delta_y = -helicopter::SPEED * delta_time,
						Some(VerticalDirection::Down) => delta_y = helicopter::SPEED * delta_time,
						None => (),
					}

					match horizontal {
						Some(HorizontalDirection::Left) => delta_x = -helicopter::SPEED * delta_time,
						Some(HorizontalDirection::Right) => delta_x = helicopter::SPEED * delta_time,
						None => (),
					}

					if vertical.is_some() && horizontal.is_some() {
						delta_x /= 2.0_f64.sqrt();
						delta_y /= 2.0_f64.sqrt();
					}

					// TODO
					for boundary in boundaries.iter() {
						if !(heli.x + delta_x >= boundary.x + boundary.width || heli.x + delta_x + helicopter::SIZE <= boundary.x || heli.y >= boundary.y + boundary.height || heli.y + helicopter::SIZE <= boundary.y) {
							if heli.x >= boundary.x + boundary.width {
								delta_x = boundary.x + boundary.width - heli.x;
							} else {
								delta_x = boundary.x - helicopter::SIZE - heli.x;
							}
						}

						if !(heli.x >= boundary.x + boundary.width || heli.x + helicopter::SIZE <= boundary.x || heli.y + delta_y >= boundary.y + boundary.height || heli.y + delta_y + helicopter::SIZE <= boundary.y) {
							if heli.y >= boundary.y + boundary.height {
								delta_y = boundary.y + boundary.height - heli.y;
							} else {
								delta_y = boundary.y - helicopter::SIZE - heli.y;
							}
						}
					}

					if delta_x != 0.0 || delta_y != 0.0 {
						heli.x += delta_x;
						heli.y += delta_y;
						
						network.send_pos(&heli);
					}
				},
				GameEvent::PlayerDeath => {
					let spawn = helicopter::find_valid_spawn(&boundaries);

					heli.x = spawn.0;
					heli.y = spawn.1;
					heli.is_dead = true;
					network.send_death();
					network.send_pos(&heli);
					death_timer = Duration::from_secs(5);
				},
				GameEvent::BulletCreation(bullet) => {
					network.send_bullet(&bullet);
					let mut bullets = network.bullets.get(&network.ip).unwrap().lock().expect("Failed to acquire lock on bullets");
					bullets.push(bullet);
					shoot_cooldown = Duration::from_millis(250);
				},
				GameEvent::BulletDestruction(ip, index) => {
					let mut bullets = network.bullets.get(&ip).unwrap().lock().expect("Failed to acquire lock on bullets");
					_ = bullets.remove(index);
				},
				GameEvent::BulletReflection(ip, index, segment) => {
					// TODO, figure out if this should be network message or computed locally
					let mut bullets = network.bullets.get(&ip).unwrap().lock().expect("Failed to acquire lock on bullets");
					let normal = segment.get_normal();
					let bullet = bullets.get_mut(index).unwrap();

					// Optimization
					let normal_length = (normal.x * normal.x + normal.y * normal.y).sqrt();
					let cos = normal.x / normal_length;
					let sin = normal.y / normal_length;
					let rotated_x = bullet.dx * cos - bullet.dy * -sin;
					let rotated_y = bullet.dx * -sin + bullet.dy * cos;
					let reflected_x = -rotated_x;
					let reflected_y = rotated_y;
					let final_x = reflected_x * cos - reflected_y * sin;
					let final_y = reflected_x * sin + reflected_y * cos;

					bullet.dx = final_x;
					bullet.dy = final_y;
				},
				GameEvent::FortCreation(fort) => {
					network.send_fort(&fort);
					let mut forts = network.forts.get(&network.ip).unwrap().lock().expect("Failed to acquire lock on network");
					forts.push(fort);
					curr_fort_index = forts.len() - 1;
				},
				GameEvent::FortHit(ip, index) => {
					let mut forts = network.forts.get(&ip).unwrap().lock().expect("Failed to acquire lock on network");
					let fort = forts.get_mut(index).unwrap();
					
					fort.health -= 1;

					if fort.health == 0 {
						let removed_fort = forts.remove(index);
						forts.iter_mut().for_each(|fort| fort.connections.retain(|connection| connection != &(removed_fort.x, removed_fort.y)));

						if curr_fort_index != 0 { curr_fort_index -= 1; }
					}
				},
				GameEvent::FortConnectionCreation(index, end) => {
					let mut forts = network.forts.get(&network.ip).unwrap().lock().expect("Failed to acquire lock on network");
					let fort = forts.get_mut(index).unwrap();
					let connection = ((fort.x, fort.y), (end.x, end.y));
					fort.connections.push((end.x, end.y));

					drop(forts);

					network.send_fort_connection(connection);
				},
				GameEvent::CurrFortChanged(index) => curr_fort_index = index,
			}
		}

		for (_, bullets) in network.bullets.iter() {
			for bullet in bullets.lock().expect("Failed to acquire lock on bullets").iter_mut() {
				bullet.update(&delta_time);
			}
		}

		if delta_time <= death_timer {
			death_timer -= delta_time;
		} else {
			death_timer = Duration::ZERO;
		}

		if delta_time <= shoot_cooldown {
			shoot_cooldown -= delta_time;
		} else {
			shoot_cooldown = Duration::ZERO;
		}

		if heli.is_dead && death_timer.is_zero() {
			heli.is_dead = false;
			network.send_death();
		}

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
			println!("TOTAL CONNECTIONS: {}", network.forts.get(&network.ip).unwrap().lock().expect("Failed to acquire lock on forts").iter().map(|fort| fort.connections.len()).reduce(|a, b| a + b).unwrap());
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
				fort.draw_connections(focus, &mut canvas, color)?;
			}

			for fort in forts.lock().expect("Failed to acquire lock on forts").iter() {
				fort.draw(focus, &mut canvas)?;
			}
		}

		let heli = heli_mutex.lock().expect("Failed to acquire lock on heli");

		if let Some(fort) = network.forts.get(&network.ip).unwrap().lock().expect("Failed to acquire lock on fort").get(curr_fort_index) {
			let heli_x = heli.x + helicopter::SIZE / 2.0;
			let heli_y = heli.y + helicopter::SIZE / 2.0;
			let fort_x = fort.x + fort::SIZE / 2.0;
			let fort_y = fort.y + fort::SIZE / 2.0;

			let segment = Segment::new(Vec2::new(fort_x, fort_y), Vec2::new(heli_x, heli_y), fort::CONNECTION_HEIGHT);
			if segment.length() < 15.0 {
				segment.draw(focus, &mut canvas, Color::RGB(255, 100, 100), 125)?;
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
		"1" => network::create_game(),
		"2" => network::connect_to_game(),
		_ => panic!("Incorrect response!")
	}
}

fn get_current_time() -> Duration {
	SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards and we're all doomed anyway")
}
