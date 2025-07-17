use rodio::Decoder;
use std::fs::File;
use std::io::BufReader;
use std::thread;

pub fn play_ogg_file1(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.to_owned();
    thread::spawn(move || {
        let stream_handle =
            rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");
        let sink = rodio::Sink::connect_new(&stream_handle.mixer());
        let file = BufReader::new(File::open(&path).unwrap());
        sink.append(Decoder::try_from(file).unwrap());
        sink.sleep_until_end();
    });
    Ok(())
}
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
