use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::helicopter::Helicopter;

pub struct Network {
    peers: Arc<Mutex<HashMap<String, TcpStream>>>,
    pub helis: HashMap<String, Arc<Mutex<Helicopter>>>,
    pub ip: String,
}

impl Network {
    pub fn new(ip: &str) -> Self {
        Network {
            peers: Arc::new(Mutex::new(HashMap::new())),
            helis: HashMap::new(),
            ip: ip.to_string(),
        }
    }

    fn send_curr_peers(&mut self, ip: &str, peer: &mut TcpStream) {
        let peers = self.peers.lock().expect("Failed to acquire lock on peers");
        let mut message = String::from("CURRPLAYERS");

        for (ip, _) in peers.iter() {
            message.push_str(&format!(" {}", ip));
        }

        peer.write_all(message.as_bytes())
            .expect("Failed to write to peer");

        println!("      SENT {} MESSAGE: \"{}\"", ip, message);
    }

    pub fn send_pos(&mut self, heli: &Helicopter) {
        let mut peers = self.peers.lock().expect("Failed to acquire lock on peers");
        for (_, player) in peers.iter_mut() {
            player
                .write_all(format!("POS {} {}", heli.x, heli.y).as_bytes())
                .expect("Failed to write to player");
        }
    }

    pub fn add_and_listen(&mut self, ip: String, peer: TcpStream) {
        let mut peers = self.peers.lock().expect("Failed to acuire lock on network");
        peers.insert(ip.clone(), peer.try_clone().expect("Failed to clone peer"));

        let heli = Arc::new(Mutex::new(Helicopter::new(0, 0, ip.clone())));
        self.helis.insert(ip, heli.clone());

        print!("DEBUG INFO:");
        for (ip, _) in peers.iter() {
            print!(" {}", ip);
        }
        println!();

        thread::spawn(move || handle_peer(peer, heli));
    }
}

pub fn start_listening_for_connection(network: Arc<Mutex<Network>>) {
    let listener = {
        let network = network.lock().expect("Failed to acquire lock on network");
        TcpListener::bind(&network.ip)
            .expect(&format!("Failed to bind to IP address {}", &network.ip))
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
                let raw_message = String::from_utf8_lossy(&buffer[..bytes_read]);
                let message: Vec<&str> = raw_message.split(" ").collect();

                match message[0] {
                    "JOIN" => {
                        println!("  RECIEVED JOIN COMMAND");
                        let mut network =
                            network.lock().expect("Failed to acquire lock on network");

                        let ip = message[1].to_string();
                        network.send_curr_peers(&ip, &mut stream);

                        network.add_and_listen(ip, stream);
                    }
                    "NEWPEER" => {
                        println!("  RECIEVED NEWPEER COMMAND");
                        let mut network =
                            network.lock().expect("Failed to acquire lock on network");

                        network.add_and_listen(message[1].to_string(), stream);
                    }
                    "CONNECT" => println!("  RECIEVED CONNECT COMMAND"),
                    _ => {
                        eprintln!(" Not a command!");
                        eprintln!(
                            " Message was: {}",
                            String::from_utf8_lossy(&buffer[..bytes_read])
                        );
                    }
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

pub fn handle_peer(mut peer: TcpStream, heli: Arc<Mutex<Helicopter>>) {
    loop {
        let mut buffer = [0; 1024];
        //println!("STARTED READING PEER");
        match peer.read(&mut buffer) {
            Ok(0) => {
                println!("BREAKING OUT OF READING LOOP");
                break;
            }
            Ok(bytes_read) => {
                let raw_message = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();

                let heli_clone = heli.clone();
                thread::spawn(move || handle_peer_message(raw_message, heli_clone));
            }
            Err(e) => {
                eprintln!("{}", e);
                break;
            }
        }
    }
}

fn handle_peer_message(message: String, heli: Arc<Mutex<Helicopter>>) {
    let message_parts: Vec<&str> = message.split(" ").collect();

    match message_parts[0] {
        "POS" => {
            /*println!(
                "  RECIEVED POS COMMAND: {} {}",
                message_parts[1], message_parts[2]
            );*/
            let mut heli = heli.lock().expect("Failed to acquire lock on heli");
            heli.x = message_parts[1].parse().expect("Invalid format");
            heli.y = match message_parts[2].parse() {
                Ok(num) => num,
                Err(_) => {
                    eprintln!("DOUBLE MESSAGE");
                    heli.y
                }
            };
        }
        _ => {
            eprintln!("Not a command!");
            eprintln!("Message was: {}", message);
        }
    }
}
