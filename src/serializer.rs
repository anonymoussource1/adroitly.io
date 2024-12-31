use std::fmt;

pub enum Message {
	Join(bool, String),
	CurrPlayers(Vec<String>),
	Pos(f64, f64),
	Bullet(f64, f64, f64, f64)
}

impl Message {
	pub fn serialize(&self) -> Vec<u8> {
		match self {
			Self::Join(is_first, ip) => {
				let mut join = vec![0b00000000 + (*is_first as u8) << 5];
				join.append(&mut serialize_ip(&ip));

				join
			}
			Self::CurrPlayers(ips) => {
				let mut currplayers = vec![0b01000000 + ips.len() as u8];

				for ip in ips {
					currplayers.append(&mut serialize_ip(ip));
				}

				currplayers
			}
			Self::Pos(x, y) => {
				let mut pos = vec![0b10000000];

				pos.append(&mut Vec::from(x.to_be_bytes()));
				pos.append(&mut Vec::from(y.to_be_bytes()));

				pos
			}
			Self::Bullet(x, y, dx, dy) => {
				let mut bullet = vec![0b11000000];

				bullet.append(&mut Vec::from(x.to_be_bytes()));
				bullet.append(&mut Vec::from(y.to_be_bytes()));
				bullet.append(&mut Vec::from(dx.to_be_bytes()));
				bullet.append(&mut Vec::from(dy.to_be_bytes()));

				bullet
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
                print!("DESERIALIZING CURRPLAYERS");
				let len = bytes[0] & 0b00111111;
				let mut ips = Vec::with_capacity(len as usize);

				for i in 0..len {
                    // 1..7
					ips.push(deserialize_ip(&bytes[((1 + 6 * i) as usize)..(1 + 6 * (i + 1) as usize)]));
				}

				Self::CurrPlayers(ips)
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

				Self::Bullet(x, y, dx, dy)
			}
			_ => unreachable!()
		}
	}

	pub fn len(&self) -> u8 {
		match self {
			Self::Join(..) => 7,
			Self::CurrPlayers(ips) => 1 + 6 * ips.len() as u8,
			Self::Pos(..) => 17,
			Self::Bullet(..) => 33
		}
	}
}

impl fmt::Display for Message {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Join(is_first, ip) => write!(f, "JOIN {} {}", is_first, ip),
			Self::CurrPlayers(ips) => {
				let mut formatted = String::new();

				for ip in ips {
					formatted.push_str(ip);
					formatted.push_str(" ");
				}

				write!(f, "CURRPLAYERS {}", formatted.trim())
			}
			Self::Pos(x, y) => write!(f, "POS {} {}", x, y),
			Self::Bullet(x, y, dx, dy) => write!(f, "BULLET {} {} {} {}", x, y, dx, dy)
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
