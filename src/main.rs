use std::fs::File;

fn main() {
    let f = File::open("./ubuntu.torrent").unwrap();

    let mut reader = beeenn::BEReader::new(f);
    let value = reader.next_value().unwrap().unwrap();

    println!("TOP: {:?}", value);

    let value = value.get("announce-list".as_bytes()).unwrap();

    println!("VALUE: {:?}", value);
}
