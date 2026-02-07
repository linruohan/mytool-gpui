use std::{fs::File, io::BufReader, thread};

use rodio::Decoder;
use tracing::error;

/// 播放ogg音频文件
pub fn play_ogg_file(path: &str) -> Result<std::thread::JoinHandle<Result<(), String>>, String> {
    let path = path.to_owned();
    // Try to open stream first to fail fast
    let stream_builder = rodio::OutputStreamBuilder::open_default_stream()
        .map_err(|e| format!("open default audio stream failed: {}", e))?;
    let handle = thread::spawn(move || {
        // Open the file inside the thread to avoid holding resources across threads
        match File::open(&path) {
            Ok(file) => {
                let sink = rodio::Sink::connect_new(stream_builder.mixer());
                let buf = BufReader::new(file);
                match Decoder::try_from(buf) {
                    Ok(source) => {
                        sink.append(source);
                        sink.sleep_until_end();
                        Ok(())
                    },
                    Err(e) => {
                        error!("audio decode failed: {:?}", e);
                        Err(format!("audio decode failed: {}", e))
                    },
                }
            },
            Err(e) => {
                error!("open audio file failed: {}", e);
                Err(format!("open audio file failed: {}", e))
            },
        }
    });
    Ok(handle)
}
