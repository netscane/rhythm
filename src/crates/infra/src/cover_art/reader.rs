use application::query::get_cover_art::{CoverArtBytes, CoverArtReader, CoverSource};
use application::query::QueryError;
use async_trait::async_trait;
use bytes::Bytes;
use std::path::Path;
use std::process::Command;

/// 封面艺术读取器实现
pub struct CoverArtReaderImpl;

impl CoverArtReaderImpl {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CoverArtReaderImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CoverArtReader for CoverArtReaderImpl {
    async fn read(&self, source: &CoverSource) -> Result<CoverArtBytes, QueryError> {
        match source {
            CoverSource::External { protocol, path } => {
                let data = self.read_external(protocol, path).await?;
                let mime_type = guess_mime_type_from_path(path);
                Ok(CoverArtBytes { data, mime_type })
            }
            CoverSource::Embedded { protocol, path } => {
                let data = self.read_embedded(protocol, path).await?;
                let mime_type = guess_mime_type_from_data(&data);
                Ok(CoverArtBytes { data, mime_type })
            }
        }
    }
}

impl CoverArtReaderImpl {
    async fn read_external(&self, protocol: &str, path: &str) -> Result<Bytes, QueryError> {
        match protocol {
            "file" | "local" | "" => {
                // 本地文件系统
                let data = tokio::fs::read(path)
                    .await
                    .map_err(|e| QueryError::ExecutionError(format!("Failed to read file {}: {}", path, e)))?;
                Ok(Bytes::from(data))
            }
            "smb" => {
                // SMB 协议暂不支持，需要额外实现
                Err(QueryError::ExecutionError(format!(
                    "SMB protocol not yet supported for cover art: {}",
                    path
                )))
            }
            _ => Err(QueryError::ExecutionError(format!(
                "Unsupported protocol: {}",
                protocol
            ))),
        }
    }

    async fn read_embedded(&self, protocol: &str, path: &str) -> Result<Bytes, QueryError> {
        match protocol {
            "file" | "local" | "" => {
                // 本地文件系统，使用 ffmpeg 提取嵌入封面
                extract_embedded_cover_ffmpeg(path).await
            }
            "smb" => {
                // SMB 协议暂不支持
                Err(QueryError::ExecutionError(format!(
                    "SMB protocol not yet supported for embedded cover: {}",
                    path
                )))
            }
            _ => Err(QueryError::ExecutionError(format!(
                "Unsupported protocol: {}",
                protocol
            ))),
        }
    }
}

/// 根据文件路径猜测 MIME 类型
fn guess_mime_type_from_path(path: &str) -> String {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg".to_string(),
        "png" => "image/png".to_string(),
        "gif" => "image/gif".to_string(),
        "webp" => "image/webp".to_string(),
        "bmp" => "image/bmp".to_string(),
        "tiff" | "tif" => "image/tiff".to_string(),
        _ => "image/jpeg".to_string(),
    }
}

/// 根据图片数据猜测 MIME 类型（magic number）
fn guess_mime_type_from_data(data: &[u8]) -> String {
    if data.len() < 4 {
        return "image/jpeg".to_string();
    }

    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg".to_string()
    } else if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        "image/png".to_string()
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        "image/gif".to_string()
    } else if data.starts_with(b"RIFF") && data.len() > 12 && &data[8..12] == b"WEBP" {
        "image/webp".to_string()
    } else if data.starts_with(&[0x42, 0x4D]) {
        "image/bmp".to_string()
    } else {
        "image/jpeg".to_string()
    }
}

/// 使用 ffmpeg 从音频文件中提取嵌入封面
async fn extract_embedded_cover_ffmpeg(audio_path: &str) -> Result<Bytes, QueryError> {
    let path = audio_path.to_string();
    
    // 使用 spawn_blocking 因为 Command 是阻塞的
    tokio::task::spawn_blocking(move || {
        // 首先尝试直接复制封面
        let output = Command::new("ffmpeg")
            .args([
                "-i", &path,
                "-an",           // 无音频
                "-vcodec", "copy", // 直接复制图片
                "-f", "image2pipe",
                "-"
            ])
            .output()
            .map_err(|e| QueryError::ExecutionError(format!("Failed to run ffmpeg: {}", e)))?;

        if output.status.success() && !output.stdout.is_empty() {
            return Ok(Bytes::from(output.stdout));
        }

        // 如果失败，尝试转码为 JPEG
        let output = Command::new("ffmpeg")
            .args([
                "-i", &path,
                "-an",
                "-c:v", "mjpeg",
                "-f", "image2pipe",
                "-vframes", "1",
                "-"
            ])
            .output()
            .map_err(|e| QueryError::ExecutionError(format!("Failed to run ffmpeg: {}", e)))?;

        if output.status.success() && !output.stdout.is_empty() {
            Ok(Bytes::from(output.stdout))
        } else {
            Err(QueryError::ExecutionError(format!(
                "ffmpeg failed to extract cover: {}",
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    })
    .await
    .map_err(|e| QueryError::ExecutionError(format!("Task join error: {}", e)))?
}
