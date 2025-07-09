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
use std::time::Duration;

use crate::bullet::Bullet;
use crate::helicopter::Helicopter;
use crate::serializer::Message;

pub struct Network {
	peers: HashMap<String, TcpStream>,
	pub bullets: HashMap<String, Arc<Mutex<Vec<Bullet>>>>,
	pub helis: HashMap<String, Arc<Mutex<Helicopter>>>,
	pub ip: String
}

impl Network {
	pub fn new(ip: &str) -> Self {
		Network {
			peers: HashMap::new(),
			bullets: HashMap::new(),
			helis: HashMap::new(),
			ip: ip.to_string()
		}
	}

	pub fn send_bullet(&mut self, bullet: &Bullet) {
		let bullet = Message::Bullet(bullet.x, bullet.y, bullet.dx, bullet.dy);
		for (_, peer) in self.peers.iter_mut() {
			peer.write_all(&bullet.serialize()).expect("Failed to write to peer");
		}
	}

	fn send_curr_peers(&mut self, peer: &mut TcpStream) {
		let curr_peers = Message::CurrPlayers(self.peers.keys().map(|s| s.to_owned()).collect());
		println!("{curr_peers}");

		peer.write_all(&curr_peers.serialize()).expect("Failed to write to peer");
	}

	pub fn send_pos(&mut self, heli: &Helicopter) {
		let pos = Message::Pos(heli.x, heli.y);
		for (_, peer) in self.peers.iter_mut() {
			peer.write_all(&pos.serialize()).expect("Failed to write to player");
		}
	}

	pub fn add_and_listen(&mut self, ip: String, peer: TcpStream) {
		self.peers.insert(ip.clone(), peer.try_clone().expect("Failed to clone peer"));

		let heli = Arc::new(Mutex::new(Helicopter::new(0, 0, ip.clone())));
		self.helis.insert(ip.clone(), heli.clone());

		let peer_bullets = Arc::new(Mutex::new(Vec::new()));

		let peer_bullets_clone = peer_bullets.clone();
		self.bullets.insert(ip, peer_bullets_clone);

		thread::spawn(move || handle_peer(peer, heli, peer_bullets));
	}
}

pub fn start_listening_for_connection(network: Arc<Mutex<Network>>) {
	let listener = {
		let network = network.lock().expect("Failed to acquire lock on network");
		TcpListener::bind(&network.ip).expect(&format!("Failed to bind to IP address {}", &network.ip))
	};
	for stream in listener.incoming() {
		println!("STARTED HANDLING STREAM");
		let mut stream = stream.expect("Failed to get player stream");
		let mut buffer = [0; 1024];

		match stream.read(&mut buffer) {
			Ok(0) => {
				println!("  BREAKING OUT OF LISTENING LOOP");
				break;
			}
			Ok(bytes_read) => {
				let message = Message::deserialize(&buffer[..bytes_read]);

				println!("RECIEVED \"{}\"", message);
				match message {
					Message::Join(is_first, ip) => {
						let mut network = network.lock().expect("Failed to acquire lock on network");
						println!("Acquire network lock");

						if is_first {
							network.send_curr_peers(&mut stream);
						}

                        {
                            let heli = network.helis.get(&network.ip).unwrap().lock().expect("Failed to get lock on helicopter");
                            let pos = Message::Pos(heli.x, heli.y);
                            thread::sleep(Duration::from_millis(500));
                            stream.write_all(&pos.serialize()).expect("Failed to write to peer");
                        }

                        if let Some(bullets) = network.bullets.get(&network.ip) {
                            for bullet in bullets.lock().expect("Failed to acquire lock on bullets").iter() {
                                let bullet = Message::Bullet(bullet.x, bullet.y, bullet.dx, bullet.dy);
                                stream.write_all(&bullet.serialize()).expect("Failed to write to peer");
                            }
                        };

						network.add_and_listen(ip.clone(), stream);
                    }
					_ => unreachable!()
				}
			}
			Err(e) => {
				eprintln!("Failed to read connection: {}", e);
				break;
			}
		}

		println!("FINISHED HANDLING STREAM");
	}

	println!("CLOSED LISTENING");
}

pub fn handle_peer(mut peer: TcpStream, heli: Arc<Mutex<Helicopter>>, bullets: Arc<Mutex<Vec<Bullet>>>) {
	loop {
		let mut buffer = [0; 1024];
		match peer.read(&mut buffer) {
			Ok(0) => {
				println!("BREAKING OUT OF READING LOOP");
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
					thread::spawn(move || handle_peer_message(message, heli, bullets));
				}
			}
			Err(e) => {
				eprintln!("{}", e);
				break;
			}
		}
	}
}

fn handle_peer_message(message: Message, heli: Arc<Mutex<Helicopter>>, bullets: Arc<Mutex<Vec<Bullet>>>) {
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
