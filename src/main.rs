use serde::{Deserialize, Serialize};

const UBUNTU_TORRENT: &[u8] = include_bytes!("../ubuntu.torrent");

#[derive(Debug, Serialize, Deserialize)]
struct Torrent<'a> {
    #[serde(borrow)]
    info: TorrentInfo<'a>,
    #[serde(rename = "magnet-info")]
    magnet_info: MagnetInfo<'a>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TorrentInfo<'a> {
    length: u64,
    name: String,
    #[serde(rename = "piece length")]
    piece_length: u64,
    pieces: &'a [u8],
}

#[derive(Debug, Serialize, Deserialize)]
struct MagnetInfo<'a> {
    #[serde(rename = "display-name")]
    display_name: String,

    info_hash: &'a [u8],
}

fn main() {
    let torrent: Torrent = beeenn::from_bytes(UBUNTU_TORRENT).unwrap();
    println!("{:?}", torrent);
}
