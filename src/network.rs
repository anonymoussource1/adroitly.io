use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::bullet::Bullet;
use crate::helicopter::Helicopter;
use crate::serializer::Message;

pub struct Network {
	pub players: HashMap<String, TcpStream>,
	pub bullets: HashMap<String, Arc<Mutex<Vec<Bullet>>>>,
	pub helis: HashMap<String, Arc<Mutex<Helicopter>>>,
	pub ip: String,
}

impl Network {
	pub fn new(ip: &str) -> Self {
		Network {
			players: HashMap::new(),
			bullets: HashMap::new(),
			helis: HashMap::new(),
			ip: ip.to_string(),
		}
	}

	pub fn send_bullet(&mut self, bullet: &Bullet) {
		let bullet = Message::Bullet(bullet.x, bullet.y, bullet.dx, bullet.dy);
		for (_, player) in self.players.iter_mut() {
			_ = player.write_all(&bullet.serialize());
		}
	}

	fn send_curr_players(&mut self, player: &mut TcpStream) {
		let curr_players = Message::CurrPlayers(self.players.keys().map(|s| s.to_owned()).collect());

		_ = player.write_all(&curr_players.serialize());
	}

	pub fn send_pos(&mut self, heli: &Helicopter) {
		let pos = Message::Pos(heli.x, heli.y);
		for (_, player) in self.players.iter_mut() {
			_ = player.write_all(&pos.serialize());
		}
	}

	pub fn add_and_listen(&mut self, ip: String, player: TcpStream) -> thread::JoinHandle<String> {
		self.players.insert(ip.clone(), player.try_clone().expect("Failed to clone player"));

		let mut heli = Arc::new(Mutex::new(Helicopter::new(0.0, 0.0, ip.clone())));
		if let Some(old_heli) = self.helis.insert(ip.clone(), heli.clone()) {
			heli = old_heli.clone();
			self.helis.insert(ip.clone(), old_heli);
		};

		let mut player_bullets = Arc::new(Mutex::new(Vec::new()));

		let player_bullets_clone = player_bullets.clone();
		if let Some(old_bullets) = self.bullets.insert(ip.clone(), player_bullets_clone) {
			player_bullets = old_bullets.clone();
			self.bullets.insert(ip.clone(), old_bullets);
		};

		thread::spawn(move || {
			handle_player(player, heli, player_bullets);
			println!("Player {} disconnected.", ip);
			ip
		})
	}
}

pub fn start_listening_for_connection(network: Arc<Mutex<Network>>) {
	let listener = {
		let network = network.lock().expect("Failed to acquire lock on network");
		TcpListener::bind(&network.ip).expect(&format!("Failed to bind to IP address {}", &network.ip))
	};
	println!("Started listening");
	for stream in listener.incoming() {
		println!("  Stream attempting to connect...");
		let mut stream = stream.expect("Failed to get player stream");
		let mut buffer = [0; 1024];

		match stream.read(&mut buffer) {
			Ok(0) => {
				println!("Something went wrong?");
				break;
			}
			Ok(bytes_read) => {
				let ip_thread;
				let message = Message::deserialize(&buffer[..bytes_read]);

				match message {
					Message::Join(is_first, ip) => {
						println!("      Recieved join message...");
						let mut network = network.lock().expect("Failed to acquire lock on network");

						if is_first {
							println!("      Sending current players...");
							network.send_curr_players(&mut stream);
						}

						{
							let heli = network.helis.get(&network.ip).unwrap().lock().expect("Failed to get lock on helicopter");
							let pos = Message::Pos(heli.x, heli.y);
							stream.write_all(&pos.serialize()).expect("Failed to write to player");
						}

						if let Some(bullets) = network.bullets.get(&network.ip) {
							for bullet in bullets.lock().expect("Failed to acquire lock on bullets").iter() {
								let bullet = Message::Bullet(bullet.x, bullet.y, bullet.dx, bullet.dy);
								stream.write_all(&bullet.serialize()).expect("Failed to write to player");
							}
						};

						ip_thread = network.add_and_listen(ip.clone(), stream);
					}
					_ => unreachable!(),
				}

				let network = network.clone();
				thread::spawn(move || {
					let ip = ip_thread.join().expect("Something, somewhere, went wrong");
					let mut network = network.lock().expect("Failed to acquire lock on network");
					network.players.remove(&ip);
					network.helis.remove(&ip);
					network.bullets.remove(&ip);
				});
			}
			Err(e) => {
				eprintln!("Failed to read connection: {}", e);
				break;
			}
		}

		println!("  Stream connected.");
	}

	println!("Finished listening.");
}

pub fn handle_player(mut player: TcpStream, heli: Arc<Mutex<Helicopter>>, bullets: Arc<Mutex<Vec<Bullet>>>) {
	loop {
		let mut buffer = [0; 1024];
		match player.read(&mut buffer) {
			Ok(0) => {
				break;
			}
			Ok(bytes_read) => {
				let mut start = 0;
				while start < bytes_read {
					let message = Message::deserialize(&buffer[start..bytes_read]);
					//println!("RECIEVED \"{}\"", message);

					start += message.len() as usize;

					let bullets = bullets.clone();
					let heli = heli.clone();
					thread::spawn(move || handle_player_message(message, heli, bullets));
				}
			}
			Err(e) => {
				eprintln!("{}", e);
				break;
			}
		}
	}
}

fn handle_player_message(message: Message, heli: Arc<Mutex<Helicopter>>, bullets: Arc<Mutex<Vec<Bullet>>>) {
	match message {
		Message::Pos(x, y) => {
			let mut heli = heli.lock().expect("Failed to acquire lock on heli");
			heli.x = x;
			heli.y = y;
		}
		Message::Bullet(x, y, dx, dy) => bullets.lock().expect("Failed to acquire lock on bullets").push(Bullet::new(x, y, dx, dy)),
		_ => unreachable!(),
	}
}
