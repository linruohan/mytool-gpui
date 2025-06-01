use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::thread;

pub fn play_ogg_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.to_owned();
    thread::spawn(move || {
        let (_stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        let file = BufReader::new(File::open(&path).unwrap());
        sink.append(Decoder::new(file).unwrap());
        sink.sleep_until_end();
    });
    Ok(())
}
