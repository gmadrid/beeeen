use serde::{Deserialize, Serialize};

const UBUNTU_TORRENT: &[u8] = include_bytes!("../ubuntu.torrent");

#[derive(Debug, Serialize, Deserialize)]
struct Torrent {}

fn main() {
    let torrent: Torrent = beeenn::from_bytes(UBUNTU_TORRENT).unwrap();
    println!("Torrent: {:?}", torrent);

    //    let f = File::open("./ubuntu.torrent").unwrap();
    //    let mut reader = beeenn::BEReader::new(f);
    //    let value = reader.next_value().unwrap().unwrap();
    //    println!("{:?}", value);
}
