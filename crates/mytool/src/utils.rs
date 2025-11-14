use std::{fs::File, io::BufReader, thread};

use rodio::Decoder;

pub fn play_ogg_file(path: &str) -> thread::JoinHandle<()> {
    let path = path.to_owned();
    thread::spawn(move || {
        let stream_handle =
            rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");
        let sink = rodio::Sink::connect_new(&stream_handle.mixer());

        let file = BufReader::new(File::open(&path).unwrap());
        sink.append(Decoder::try_from(file).unwrap());
        sink.sleep_until_end();
    })
}
