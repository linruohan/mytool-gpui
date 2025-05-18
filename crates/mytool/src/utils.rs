use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;

pub fn play_ogg_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 获取音频输出流
    let (_stream, stream_handle) = OutputStream::try_default()?;

    // 2. 创建播放控制器
    let sink = Sink::try_new(&stream_handle)?;

    // 3. 加载OGG文件
    let file = BufReader::new(File::open(path)?);
    let source = Decoder::new(file)?;

    // 4. 播放音频
    sink.append(source);
    sink.sleep_until_end(); // 阻塞直到播放完成

    Ok(())
}
