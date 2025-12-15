use super::rule_engine::{MetadataRuleEngine, RuleContext};
use application::command::media_parse::AudioMetadataReader;
use application::error::AppError;
use domain::value::AudioMetadata;
use id3::Tag;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct AudioMetadataReaderImpl {
    rule_engine: Arc<MetadataRuleEngine>,
}

impl AudioMetadataReaderImpl {
    pub fn new() -> Self {
        Self {
            rule_engine: Arc::new(MetadataRuleEngine::with_default_rules()),
        }
    }

    /// 使用自定义规则引擎创建
    pub fn with_rule_engine(rule_engine: MetadataRuleEngine) -> Self {
        Self {
            rule_engine: Arc::new(rule_engine),
        }
    }
}

#[async_trait::async_trait]
impl AudioMetadataReader for AudioMetadataReaderImpl {
    async fn parse(&self, path: PathBuf) -> Result<AudioMetadata, AppError> {
        // Prepare a local filesystem path to parse. For non-local protocols,
        // materialize the content into a temporary file first.

        let file = taglib::File::new(path.as_path()).map_err(|e| {
            AppError::ParseAudioMetadataError(format!("Failed to open file: {:?}", e))
        })?;

        let tag = file.tag().map_err(|e| {
            AppError::ParseAudioMetadataError(format!("Failed to read tags: {:?}", e))
        })?;

        let properties = file.audioproperties().map_err(|e| {
            AppError::ParseAudioMetadataError(format!("Failed to read properties: {:?}", e))
        })?;

        let id3_tag = Tag::read_from_path(path.as_path()).ok();

        let title = tag.title().unwrap_or_default();
        let artist_raw = tag.artist().unwrap_or_default();
        let album = tag.album().unwrap_or_default();
        let genre = tag.genre().unwrap_or_default();

        let year = tag.year();
        let track_num = tag.track();
        let lyrics = id3_tag
            .as_ref()
            .and_then(|tag| tag.lyrics().next().map(|l| l.text.clone()));

        // 使用规则引擎处理元数据
        let mut ctx = RuleContext::new(
            title,
            artist_raw,
            album,
            genre,
            year.and_then(|y| if y > 0 { Some(y as i32) } else { None }),
            track_num.and_then(|n| if n > 0 { Some(n as i32) } else { None }),
        );

        self.rule_engine.execute(&mut ctx);

        Ok(AudioMetadata {
            title: ctx.title,
            participants: ctx.artists,
            album: ctx.album,
            genres: ctx.genres,
            track_number: ctx.track_number,
            year: ctx.year,
            duration: properties.length() as i64,
            bit_rate: properties.bitrate() as i32,
            sample_rate: properties.samplerate() as i32,
            channels: properties.channels() as i32,
            picture: id3_tag
                .as_ref()
                .and_then(|tag| tag.pictures().next().map(|p| p.data.clone())),
            lyrics,
        })
    }
    async fn get_picture(&self, path: PathBuf) -> Result<Vec<u8>, AppError> {
        let id3_tag = Tag::read_from_path(path.as_path()).ok();
        let picture = id3_tag
            .as_ref()
            .and_then(|tag| tag.pictures().next().map(|p| p.data.clone()));
        Ok(picture.unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use walkdir::WalkDir;

    /// 测试遍历 /data/share/Music_folder 目录并解析所有音频文件
    /// 忽略错误，计算总耗时
    #[tokio::test]
    #[ignore] // 默认忽略，需要手动运行
    async fn test_parse_music_folder() {
        let reader = AudioMetadataReaderImpl::new();
        let music_folder = "/data/share/Music";

        // 音频文件扩展名列表
        let audio_extensions: Vec<&str> = vec![
            "mp3", "wav", "flac", "aac", "ogg", "wma", "m4a", "opus", "aiff", "au", "ra",
        ];

        let start = Instant::now();
        let mut total_files = 0;
        let mut success_count = 0;
        let mut error_count = 0;

        println!("开始遍历目录: {} (递归模式)", music_folder);

        for entry in WalkDir::new(music_folder)
            .follow_links(false) // 不跟随符号链接，避免循环
            .max_depth(100) // 限制最大递归深度，防止过深
            .into_iter()
            .filter_map(|e| e.ok())
        {
            // 只处理文件，跳过目录（walkdir 会递归遍历所有子目录）
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();

            // 检查是否为音频文件
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if !audio_extensions.contains(&ext_str.as_str()) {
                    continue;
                }
            } else {
                continue;
            }
            //println!("Processing file: {}", path.display());

            total_files += 1;

            // 解析文件，忽略错误
            match reader.parse(path.to_path_buf()).await {
                Ok(_metadata) => {
                    success_count += 1;
                    if total_files % 100 == 0 {
                        println!(
                            "已处理 {} 个文件，成功: {}, 失败: {}",
                            total_files, success_count, error_count
                        );
                    }
                }
                Err(_e) => {
                    error_count += 1;
                    // 忽略错误，不打印
                }
            }
        }

        let elapsed = start.elapsed();

        println!("\n=== 解析完成 ===");
        println!("总文件数: {}", total_files);
        println!("成功: {}", success_count);
        println!("失败: {}", error_count);
        println!("总耗时: {:.2} 秒", elapsed.as_secs_f64());
        if total_files > 0 {
            println!(
                "平均每个文件: {:.2} 毫秒",
                elapsed.as_millis() as f64 / total_files as f64
            );
        }
    }
}
