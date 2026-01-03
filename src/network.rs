use std::collections::HashMap;
use std::io::{
	Read,
	Write
};
use std::net::{
	TcpListener,
	TcpStream
};
use std::sync::{
	Arc,
	Mutex
};
use std::thread;

use crate::bullet::Bullet;
use crate::fort::Fort;
use crate::helicopter::Helicopter;
use crate::serializer::Message;

pub struct Network {
	pub players: HashMap<String, TcpStream>,
	pub bullets: HashMap<String, Arc<Mutex<Vec<Bullet>>>>,
	pub helis: HashMap<String, Arc<Mutex<Helicopter>>>,
	pub forts: HashMap<String, Arc<Mutex<Vec<Fort>>>>,
	pub ip: String
}

impl Network {
	pub fn new(ip: &str) -> Self {
		let mut network = Self {
			players: HashMap::new(),
			bullets: HashMap::new(),
			helis: HashMap::new(),
			forts: HashMap::new(),
			ip: ip.to_string()
		};

		network.bullets.insert(String::from(ip), Arc::new(Mutex::new(Vec::new())));
		network.forts.insert(String::from(ip), Arc::new(Mutex::new(Vec::new())));

		network
	}

	pub fn send_bullet(&mut self, bullet: &Bullet) {
		let bullet = Message::Bullet(bullet.x, bullet.y, bullet.dx, bullet.dy, bullet.age.as_millis());
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

	pub fn send_death(&mut self) {
		let death = Message::Death;
		for (_, player) in self.players.iter_mut() {
			_ = player.write_all(&death.serialize());
		}
	}

	pub fn send_fort(&mut self, fort: &Fort) {
		let fort = Message::Fort(fort.x, fort.y);
		for (_, player) in self.players.iter_mut() {
			_ = player.write_all(&fort.serialize());
		}
	}

	pub fn send_fort_connection(&mut self, connection: ((f64, f64), (f64, f64))) {
		let connection = Message::FortConnection(connection.0.0, connection.0.1, connection.1.0, connection.1.1);
		for (_, player) in self.players.iter_mut() {
			_ = player.write_all(&connection.serialize());
		}
	}

	pub fn add_and_listen(&mut self, ip: String, player: TcpStream) -> thread::JoinHandle<String> {
		self.players.insert(ip.clone(), player.try_clone().expect("Failed to clone player"));

		let heli = Arc::new(Mutex::new(Helicopter::new(0.0, 0.0)));
		self.helis.insert(ip.clone(), heli.clone());

		let bullets = Arc::new(Mutex::new(Vec::new()));
		self.bullets.insert(ip.clone(), bullets.clone());

		let forts = Arc::new(Mutex::new(Vec::new()));
		self.forts.insert(ip.clone(), forts.clone());

		thread::spawn(move || {
			handle_player(player, heli, bullets, forts);
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
				let Some(message) = Message::deserialize(&buffer[..bytes_read]) else { continue };

				match message {
					Message::Join(is_first, ip) => {
						println!("      Recieved join message from {}...", ip);
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

						// ?
						if let Some(bullets) = network.bullets.get(&network.ip) {
							for bullet in bullets.lock().expect("Failed to acquire lock on bullets").iter() {
								let bullet = Message::Bullet(bullet.x, bullet.y, bullet.dx, bullet.dy, bullet.age.as_millis());
								stream.write_all(&bullet.serialize()).expect("Failed to write to player");
							}
						};

						ip_thread = network.add_and_listen(ip.clone(), stream);
					}
					_ => unreachable!()
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

pub fn handle_player(mut player: TcpStream, heli: Arc<Mutex<Helicopter>>, bullets: Arc<Mutex<Vec<Bullet>>>, forts: Arc<Mutex<Vec<Fort>>>) {
	loop {
		let mut buffer = [0; 1024];
		match player.read(&mut buffer) {
			Ok(0) => {
				break;
			}
			Ok(bytes_read) => {
				let mut start = 0;
				while start < bytes_read {
					let Some(message) = Message::deserialize(&buffer[start..bytes_read]) else { continue };
					start += message.len() as usize;

					let bullets = bullets.clone();
					let heli = heli.clone();
					let forts = forts.clone();
					thread::spawn(move || handle_player_message(message, heli, bullets, forts));
				}
			}
			Err(e) => {
				eprintln!("{}", e);
				break;
			}
		}
	}
}

fn handle_player_message(message: Message, heli: Arc<Mutex<Helicopter>>, bullets: Arc<Mutex<Vec<Bullet>>>, forts: Arc<Mutex<Vec<Fort>>>) {
	match message {
		Message::Pos(x, y) => {
			let mut heli = heli.lock().expect("Failed to acquire lock on heli");
			heli.x = x;
			heli.y = y;
		}
		Message::Bullet(x, y, dx, dy, age) => {
			bullets.lock().expect("Failed to acquire lock on bullets").push(Bullet::new(x, y, dx, dy, age as u64));
		}
		Message::Death => {
			let mut heli = heli.lock().expect("Failed to acquire lock on heli");
			heli.is_dead = !heli.is_dead;
		}
		Message::Fort(x, y) => {
			let mut forts = forts.lock().expect("Failed to acquire lock on forts");
			if let Some((index, _)) = forts.iter().enumerate().find(|fort| fort.1.x == x && fort.1.y == y) {
				forts.remove(index);
				forts.iter_mut().for_each(|fort| fort.connections.retain(|connection| !(connection.0 == x && connection.1 == y)));
			} else {
				forts.push(Fort::new(x, y));
			}
		}
		Message::FortConnection(x1, y1, x2, y2) => {
			let mut forts = forts.lock().expect("Failed to acquire lock on forts");
			let fort1 = forts.iter_mut().find(|fort| fort.x == x1 && fort.y == y1).unwrap();

			fort1.connections.push((x2, y2));
		}
		_ => panic!("Recieved unsupported message during game")
	}
}
