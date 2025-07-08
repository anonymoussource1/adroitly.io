use std::fmt;
use std::convert::TryInto;
use std::collections::HashMap;
use crate::bullet::Bullet;

pub enum Message {
	Join(bool, String),
	GameState(HashMap<String, Vec<Message>>),
	Pos(f64, f64),
	Bullet(Bullet),
}

impl Message {
	pub fn serialize(&self) -> Vec<u8> {
		match self {
			Self::Join(is_first, ip) => {
				let mut join = vec![0b00000000 + (*is_first as u8) << 5];
				join.append(&mut serialize_ip(&ip));

				join
			}
			Self::GameState(state) => {
				let mut gamestate = vec![0b01000000 + state.len() as u8];

				for (ip, bullets) in state {
					gamestate.append(&mut serialize_ip(ip));
                    gamestate.push(bullets.len() as u8);
                    for bullet in bullets {
                        gamestate.append(&mut bullet.serialize());
                    }
				}

				gamestate
			}
			Self::Pos(x, y) => {
				let mut pos = vec![0b10000000];

				pos.append(&mut Vec::from(x.to_be_bytes()));
				pos.append(&mut Vec::from(y.to_be_bytes()));

				pos
			}
			Self::Bullet(bullet) => {
				let mut bullet_mes = vec![0b11000000];

				bullet_mes.append(&mut Vec::from(bullet.x.to_be_bytes()));
				bullet_mes.append(&mut Vec::from(bullet.y.to_be_bytes()));
				bullet_mes.append(&mut Vec::from(bullet.dx.to_be_bytes()));
				bullet_mes.append(&mut Vec::from(bullet.dy.to_be_bytes()));

				bullet_mes
			}
		}
	}

	pub fn deserialize(bytes: &[u8]) -> Self {
		match bytes[0] >> 6 {
			0 => {
				let is_first = ((bytes[0] & 0b00100000) >> 5) != 0;
				let ip = deserialize_ip(&bytes[1..]);

				Self::Join(is_first, ip)
			}
			1 => {
				let ip_len = bytes[0] & 0b00111111;
                let mut state = HashMap::new();
                let mut last_pos = 1;

				for _ in 0..ip_len {
                    let ip = deserialize_ip(&bytes[last_pos..=(last_pos + 5)]);
                    let bullet_len = &bytes[last_pos + 6];
                    last_pos += 7;
                    let mut bullets = Vec::with_capacity(*bullet_len as usize);
                    for _ in 0..*bullet_len {
                        bullets.push(Self::deserialize(&bytes[(last_pos)..=(last_pos + 32)]));
                        last_pos += 33;
                    }

                    state.insert(ip, bullets);
				}

				Self::GameState(state)
			}
			2 => {
				let x = f64::from_be_bytes(bytes[1..=8].try_into().expect("Slice is incorrect length"));
				let y = f64::from_be_bytes(bytes[9..=16].try_into().expect("Slice is incorrect length"));

				Self::Pos(x, y)
			}
			3 => {
				let x = f64::from_be_bytes(bytes[1..=8].try_into().expect("Slice is incorrect length"));
				let y = f64::from_be_bytes(bytes[9..=16].try_into().expect("Slice is incorrect length"));
				let dx = f64::from_be_bytes(bytes[17..=24].try_into().expect("Slice is incorrect length"));
				let dy = f64::from_be_bytes(bytes[25..=32].try_into().expect("Slice is incorrect length"));

				Self::Bullet(Bullet::new(x, y, dx, dy))
			}
			_ => unreachable!()
		}
	}

	pub fn len(&self) -> u8 {
		match self {
			Self::Join(..) => 7,
			Self::GameState(..) => todo!(),
			Self::Pos(..) => 17,
			Self::Bullet(..) => 33
		}
	}
}

impl fmt::Display for Message {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Join(is_first, ip) => write!(f, "JOIN {} {}", is_first, ip),
			Self::GameState(state) => {
				let mut formatted = String::new();

				for (ip, bullets) in state {
					formatted.push_str(ip);
					formatted.push_str(" [");

                    for bullet in bullets {
                        formatted.push_str(&format!("\"{}\", ", bullet));
                    }

                    formatted.push_str("], ");
				}

				write!(f, "CURRPLAYERS {}", formatted.trim())
			}
			Self::Pos(x, y) => write!(f, "POS {} {}", x, y),
			Self::Bullet(bullet) => write!(f, "BULLET {} {} {} {}", bullet.x, bullet.y, bullet.dx, bullet.dy)
		}
	}
}

fn serialize_ip(ip: &str) -> Vec<u8> {
	let ip_parts: Vec<_> = ip.split(':').collect();
	let ip_secs: Vec<u8> = ip_parts[0].split('.').map(|x| x.parse().expect("Failed to parse ip section")).collect();
	let port: u16 = ip_parts[1].parse().expect("Failed to parse port");
	let mut serialized = vec![ip_secs[0], ip_secs[1], ip_secs[2], ip_secs[3]];
	serialized.append(&mut Vec::from(port.to_be_bytes()));

	serialized
}

fn deserialize_ip(ip_bytes: &[u8]) -> String {
	format!("{}.{}.{}.{}:{}", ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3], u16::from_be_bytes([ip_bytes[4], ip_bytes[5]]))
}
