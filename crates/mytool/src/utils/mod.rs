use std::io::{BufReader, Cursor};
use std::thread;

use rodio::Decoder;
use tracing::error;

/// 编译期内嵌的成功提示音
///
/// 内嵌后单 exe 分发无需外部 assets 目录；若未来需要自定义音效，
/// 可在 exe 同目录放置文件并恢复按路径加载的逻辑
pub const SUCCESS_OGG: &[u8] = include_bytes!("../../../../assets/sounds/success.ogg");

/// 播放内嵌的成功提示音 ogg 音频
///
/// 在独立线程中异步播放，立即返回线程句柄；打开音频流失败时快速报错，
/// 解码/播放失败仅记录日志，不阻塞 UI
pub fn play_success_sound() -> Result<std::thread::JoinHandle<Result<(), String>>, String> {
    // 先打开音频流以便快速失败（例如无音频设备时直接返回错误）
    let stream_builder = rodio::OutputStreamBuilder::open_default_stream()
        .map_err(|e| format!("open default audio stream failed: {}", e))?;
    let handle = thread::spawn(move || {
        let sink = rodio::Sink::connect_new(stream_builder.mixer());
        // Cursor 包装内嵌字节数据，满足解码器要求的 Read + Seek
        let buf = BufReader::new(Cursor::new(SUCCESS_OGG));
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
    });
    Ok(handle)
}
