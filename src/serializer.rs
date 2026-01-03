use std::fmt;

pub enum Message {
	Join(bool, String),
	CurrPlayers(Vec<String>),
	Pos(f64, f64),
	Bullet(f64, f64, f64, f64, u128),
	Death,
	Fort(f64, f64),
	FortConnection(f64, f64, f64, f64)
}

impl Message {
	pub fn serialize(&self) -> Vec<u8> {
		match self {
			Self::Join(is_first, ip) => {
				let mut join = Vec::with_capacity(8);
				join.push(0);
				join.push(*is_first as u8);
				join.append(&mut serialize_ip(&ip));

				join
			}
			Self::CurrPlayers(ips) => {
				let mut players = Vec::with_capacity(2 + 6 * ips.len());
				players.push(1);
				players.push(ips.len() as u8);

				for ip in ips {
					players.append(&mut serialize_ip(ip));
				}

				players
			}
			Self::Pos(x, y) => {
				let mut pos = Vec::with_capacity(17);

				pos.push(2);
				pos.append(&mut Vec::from(x.to_be_bytes()));
				pos.append(&mut Vec::from(y.to_be_bytes()));

				pos
			}
			Self::Bullet(x, y, dx, dy, age) => {
				let mut bullet = Vec::with_capacity(49);

				bullet.push(3);
				bullet.append(&mut Vec::from(x.to_be_bytes()));
				bullet.append(&mut Vec::from(y.to_be_bytes()));
				bullet.append(&mut Vec::from(dx.to_be_bytes()));
				bullet.append(&mut Vec::from(dy.to_be_bytes()));
				bullet.append(&mut Vec::from(age.to_be_bytes()));

				bullet
			}
			Self::Death => {
				vec![4]
			}
			Self::Fort(x, y) => {
				let mut fort = Vec::with_capacity(17);

				fort.push(5);
				fort.append(&mut Vec::from(x.to_be_bytes()));
				fort.append(&mut Vec::from(y.to_be_bytes()));

				fort
			}
			Self::FortConnection(x1, y1, x2, y2) => {
				let mut fort_connection = Vec::with_capacity(33);

				fort_connection.push(6);
				fort_connection.append(&mut Vec::from(x1.to_be_bytes()));
				fort_connection.append(&mut Vec::from(y1.to_be_bytes()));
				fort_connection.append(&mut Vec::from(x2.to_be_bytes()));
				fort_connection.append(&mut Vec::from(y2.to_be_bytes()));

				fort_connection
			}
		}
	}

	pub fn deserialize(bytes: &[u8]) -> Option<Self> {
		match bytes[0] {
			0 => {
				let is_first = bytes[1] != 0;
				let ip = deserialize_ip(&bytes[2..]);

				Some(Self::Join(is_first, ip))
			}
			1 => {
				let ip_len = bytes[1];
				let mut ips = Vec::with_capacity(ip_len as usize);

				for i in 0..ip_len {
					let ip = deserialize_ip(&bytes[(2 + i as usize * 6)..(2 + (i as usize + 1) * 6)]);

					ips.push(ip);
				}

				Some(Self::CurrPlayers(ips))
			}
			2 => {
				let x = f64::from_be_bytes(bytes[1..=8].try_into().expect("Slice is incorrect length"));
				let y = f64::from_be_bytes(bytes[9..=16].try_into().expect("Slice is incorrect length"));

				Some(Self::Pos(x, y))
			}
			3 => {
				let x = f64::from_be_bytes(bytes[1..=8].try_into().expect("Slice is incorrect length"));
				let y = f64::from_be_bytes(bytes[9..=16].try_into().expect("Slice is incorrect length"));
				let dx = f64::from_be_bytes(bytes[17..=24].try_into().expect("Slice is incorrect length"));
				let dy = f64::from_be_bytes(bytes[25..=32].try_into().expect("Slice is incorrect length"));
				let age = u128::from_be_bytes(bytes[33..=48].try_into().expect("Slice is incorrect length"));

				Some(Self::Bullet(x, y, dx, dy, age))
			}
			4 => Some(Self::Death),
			5 => {
				let x = f64::from_be_bytes(bytes[1..=8].try_into().expect("Slice is incorrect length"));
				let y = f64::from_be_bytes(bytes[9..=16].try_into().expect("Slice is incorrect length"));

				Some(Self::Fort(x, y))
			}
			6 => {
				let x1 = f64::from_be_bytes(bytes[1..=8].try_into().expect("Slice is incorrect length"));
				let y1 = f64::from_be_bytes(bytes[9..=16].try_into().expect("Slice is incorrect length"));
				let x2 = f64::from_be_bytes(bytes[17..=24].try_into().expect("Slice is incorrect length"));
				let y2 = f64::from_be_bytes(bytes[25..=32].try_into().expect("Slice is incorrect length"));

				Some(Self::FortConnection(x1, y1, x2, y2))
			}
			_ => {
				eprintln!("Recieved invalid message: {:?}", bytes);
				None
			}
		}
	}

	pub fn len(&self) -> u8 {
		match self {
			Self::Join(..) => 8,
			Self::CurrPlayers(ips) => 2 + 6 * ips.len() as u8,
			Self::Pos(..) => 17,
			Self::Bullet(..) => 49,
			Self::Death => 1,
			Self::Fort(..) => 17,
			Self::FortConnection(..) => 33
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
			Self::Bullet(x, y, dx, dy, age) => write!(f, "BULLET {} {} {} {} {}", x, y, dx, dy, age),
			Self::Death => write!(f, "DEATH"),
			Self::Fort(x, y) => write!(f, "FORT {} {}", x, y),
			Self::FortConnection(x1, y1, x2, y2) => write!(f, "FORT_CONNECTION {} {} {} {}", x1, y1, x2, y2)
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
