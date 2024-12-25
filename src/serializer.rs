pub enum Message {
    Join(String),
    CurrPlayers(Vec<String>),
    NewPeer(String),
    Pos(f64, f64),
    Bullet(f64, f64, f64, f64),
}

impl Message {
    pub fn serialize(&self) -> String {
        match self {
            Self::Join(ip) => {
                let ip_parts: Vec<_> = ip.split(':').collect();
                let ip_secs: Vec<u8> = ip_parts[0].split('.').map(|x| x.parse().expect("Failed to parse ip section")).collect();
                let port: u16 = ip_parts[1].parse().expect("Failed to parse port");
                let mut join = vec![0b01000001, ip_secs[0], ip_secs[1], ip_secs[2], ip_secs[3]];
                join.append(&mut Vec::from(port.to_be_bytes()));

                join
            }
            Self::CurrPlayers

}
pub fn serialize_join(ip: &str) -> Vec<u8> {
}

pub fn deserialize_join(ip_bytes: Vec<u8>) -> String {
    format!("{}.{}.{}.{}:{}", ip_bytes[1], ip_bytes[2], ip_bytes[3], ip_bytes[4], u16::from_be_bytes(ip_bytes[5..].try_into().expect("Failed to convert slice to array")))
}

pub fn serialize
