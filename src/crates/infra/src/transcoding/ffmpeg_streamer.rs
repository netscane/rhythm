use async_trait::async_trait;
use bytes::Bytes;
use domain::transcoding::{TranscodingError, TranscodingStreamer};
use futures::Stream;
use std::collections::HashMap;
use std::pin::Pin;
use std::process::Stdio;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
use tokio::process::{Child, Command};

pub struct FfmpegStreamer {
    ffmpeg_path: String,
    chunk_size: usize,
}

impl FfmpegStreamer {
    pub fn new(ffmpeg_path: String, chunk_size: usize) -> Self {
        Self {
            ffmpeg_path,
            chunk_size: chunk_size.max(4096), // 至少 4KB 的块大小
        }
    }

    /// 构建 FFmpeg 音频转码参数
    fn build_audio_arguments(
        &self,
        input_path: &str,
        output_format: &str,
        bit_rate: i32,
        additional_params: &HashMap<String, String>,
    ) -> Vec<String> {
        let mut args = vec![
            "-hide_banner".to_string(),
            "-loglevel".to_string(),
            "error".to_string(),
            "-i".to_string(),
            input_path.to_string(),
            "-vn".to_string(), // 禁用视频
        ];

        // 根据输出格式选择编码器
        let (codec, container) = match output_format.to_lowercase().as_str() {
            "mp3" => ("libmp3lame", "mp3"),
            "aac" | "m4a" => ("aac", "adts"),
            "opus" => ("libopus", "opus"),
            "ogg" | "oga" => ("libvorbis", "ogg"),
            "flac" => ("flac", "flac"),
            "wav" => ("pcm_s16le", "wav"),
            _ => ("libmp3lame", "mp3"), // 默认 mp3
        };

        args.push("-c:a".to_string());
        args.push(codec.to_string());

        // 设置比特率（无损格式不需要）
        if !["flac", "wav"].contains(&output_format.to_lowercase().as_str()) && bit_rate > 0 {
            args.push("-b:a".to_string());
            args.push(format!("{}k", bit_rate));
        }

        // 应用附加参数
        for (key, value) in additional_params {
            let param_key = if key.starts_with('-') {
                key.clone()
            } else {
                format!("-{}", key)
            };
            args.push(param_key);
            if !value.is_empty() {
                args.push(value.clone());
            }
        }

        // 输出格式
        args.push("-f".to_string());
        args.push(container.to_string());

        // 输出到 stdout
        args.push("pipe:1".to_string());

        args
    }

    /// 执行转码并返回完整数据（用于缓存）
    pub async fn transcode_to_bytes(
        &self,
        input_path: &str,
        output_format: &str,
        bit_rate: i32,
        additional_params: &HashMap<String, String>,
    ) -> Result<Bytes, TranscodingError> {
        let args =
            self.build_audio_arguments(input_path, output_format, bit_rate, additional_params);

        log::info!(
            "[FFmpeg] Executing transcode: input={}, format={}, bitrate={}k",
            input_path, output_format, bit_rate
        );
        log::debug!("[FFmpeg] Command: {} {}", self.ffmpeg_path, args.join(" "));

        let start_time = std::time::Instant::now();

        let output = Command::new(&self.ffmpeg_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                log::error!("[FFmpeg] Failed to execute: {}", e);
                TranscodingError::ExecutionErr(format!("Failed to execute FFmpeg: {}", e))
            })?;

        let elapsed = start_time.elapsed();

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::error!(
                "[FFmpeg] Transcode failed: input={}, exit_code={:?}, stderr={}",
                input_path,
                output.status.code(),
                stderr
            );
            return Err(TranscodingError::TranscodingErr(format!(
                "FFmpeg failed: {}",
                stderr
            )));
        }

        log::info!(
            "[FFmpeg] Transcode success: input={}, output_size={} bytes, elapsed={:?}",
            input_path,
            output.stdout.len(),
            elapsed
        );

        Ok(Bytes::from(output.stdout))
    }
}

/// FFmpeg 输出流
pub struct FfmpegOutputStream {
    child: Child,
    stdout: Option<tokio::process::ChildStdout>,
    chunk_size: usize,
    buffer: Vec<u8>,
}

impl FfmpegOutputStream {
    fn new(mut child: Child, chunk_size: usize) -> Result<Self, TranscodingError> {
        let stdout = child.stdout.take().ok_or_else(|| {
            TranscodingError::ExecutionErr("Failed to capture FFmpeg stdout".to_string())
        })?;

        Ok(Self {
            child,
            stdout: Some(stdout),
            chunk_size,
            buffer: vec![0u8; chunk_size],
        })
    }
}

impl Stream for FfmpegOutputStream {
    type Item = Result<Vec<u8>, TranscodingError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        let stdout = match this.stdout.as_mut() {
            Some(s) => s,
            None => return Poll::Ready(None),
        };

        let mut read_buf = ReadBuf::new(&mut this.buffer);

        match Pin::new(stdout).poll_read(cx, &mut read_buf) {
            Poll::Ready(Ok(())) => {
                let filled = read_buf.filled();
                if filled.is_empty() {
                    // EOF
                    this.stdout = None;
                    Poll::Ready(None)
                } else {
                    Poll::Ready(Some(Ok(filled.to_vec())))
                }
            }
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(TranscodingError::ExecutionErr(
                format!("FFmpeg read error: {}", e),
            )))),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for FfmpegOutputStream {
    fn drop(&mut self) {
        // 尝试终止 FFmpeg 进程
        if let Err(e) = self.child.start_kill() {
            log::debug!("Failed to kill FFmpeg process: {}", e);
        }
    }
}

#[async_trait]
impl TranscodingStreamer for FfmpegStreamer {
    async fn create_stream(
        &self,
        input_path: String,
        output_format: String,
        bit_rate: i32,
        additional_params: HashMap<String, String>,
    ) -> Result<
        Box<dyn Stream<Item = Result<Vec<u8>, TranscodingError>> + Unpin + Send>,
        TranscodingError,
    > {
        let args =
            self.build_audio_arguments(&input_path, &output_format, bit_rate, &additional_params);

        log::info!(
            "[FFmpeg] Creating stream: input={}, format={}, bitrate={}k, chunk_size={}",
            input_path, output_format, bit_rate, self.chunk_size
        );
        log::debug!("[FFmpeg] Stream command: {} {}", self.ffmpeg_path, args.join(" "));

        let child = Command::new(&self.ffmpeg_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                log::error!("[FFmpeg] Failed to start process: {}", e);
                TranscodingError::ExecutionErr(format!("Failed to start FFmpeg: {}", e))
            })?;

        log::debug!("[FFmpeg] Process started, pid={:?}", child.id());

        let stream = FfmpegOutputStream::new(child, self.chunk_size)?;
        Ok(Box::new(stream))
    }
}
