use crate::query::dao::AudioFileDao;
use crate::query::stream_cache::{
    generate_cache_key, generate_raw_cache_key, StreamCache, StreamCacheConfig, StreamCacheData,
};
use crate::query::QueryError;
use bytes::Bytes;
use domain::transcoding::{TranscodingError, TranscodingStreamer};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

/// 流媒体信息（用于 stream API）
#[derive(Debug, Clone)]
pub struct StreamInfo {
    /// 文件路径
    pub path: String,
    /// 文件大小（字节）
    pub size: i64,
    /// 文件后缀（如 mp3, flac）
    pub suffix: String,
    /// 比特率（kbps）
    pub bit_rate: i32,
    /// 时长（秒）
    pub duration: i64,
    /// MIME 类型
    pub content_type: String,
}

impl StreamInfo {
    /// 根据文件后缀获取 MIME 类型
    pub fn mime_type_from_suffix(suffix: &str) -> String {
        match suffix.to_lowercase().as_str() {
            "mp3" => "audio/mpeg".to_string(),
            "flac" => "audio/flac".to_string(),
            "ogg" | "oga" => "audio/ogg".to_string(),
            "opus" => "audio/opus".to_string(),
            "m4a" | "aac" => "audio/mp4".to_string(),
            "wav" => "audio/wav".to_string(),
            "wma" => "audio/x-ms-wma".to_string(),
            "aiff" | "aif" => "audio/aiff".to_string(),
            "ape" => "audio/ape".to_string(),
            "dsf" => "audio/dsf".to_string(),
            "dff" => "audio/dff".to_string(),
            "wv" => "audio/wavpack".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    }
}

/// stream 请求参数
#[derive(Debug, Clone)]
pub struct StreamRequest {
    /// 文件 ID
    pub id: i64,
    /// 最大比特率（可选，kbps）
    pub max_bit_rate: Option<i32>,
    /// 目标格式（可选，如 mp3, raw）
    pub format: Option<String>,
    /// 时间偏移（秒，仅视频或支持 Transcode Offset 扩展时有效）
    pub time_offset: Option<i32>,
    /// 是否估算 Content-Length
    pub estimate_content_length: bool,
}

/// 转码决策结果
#[derive(Debug, Clone)]
pub struct TranscodeDecision {
    /// 是否需要转码
    pub needs_transcoding: bool,
    /// 目标格式
    pub target_format: String,
    /// 目标比特率
    pub target_bit_rate: i32,
    /// MIME 类型
    pub content_type: String,
    /// 缓存键
    pub cache_key: String,
    /// 估算的输出大小（字节），用于设置 Content-Length
    pub estimated_size: Option<u64>,
}

/// 流媒体响应数据
#[derive(Debug)]
pub struct StreamData {
    /// 音频数据
    pub data: Bytes,
    /// MIME 类型
    pub content_type: String,
    /// 数据大小
    pub size: u64,
    /// 是否来自缓存
    pub from_cache: bool,
}

#[derive(Clone)]
pub struct StreamMedia {
    dao: Arc<dyn AudioFileDao + Send + Sync>,
    cache: Option<Arc<dyn StreamCache + Send + Sync>>,
    config: Option<Arc<dyn StreamCacheConfig + Send + Sync>>,
    transcoder: Option<Arc<dyn TranscodingStreamer + Send + Sync>>,
}

impl StreamMedia {
    pub fn new(dao: Arc<dyn AudioFileDao + Send + Sync>) -> Self {
        Self {
            dao,
            cache: None,
            config: None,
            transcoder: None,
        }
    }

    pub fn with_cache(mut self, cache: Arc<dyn StreamCache + Send + Sync>) -> Self {
        self.cache = Some(cache);
        self
    }

    pub fn with_config(mut self, config: Arc<dyn StreamCacheConfig + Send + Sync>) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_transcoder(mut self, transcoder: Arc<dyn TranscodingStreamer + Send + Sync>) -> Self {
        self.transcoder = Some(transcoder);
        self
    }

    /// 获取流媒体信息
    pub async fn get_stream_info(&self, request: &StreamRequest) -> Result<StreamInfo, QueryError> {
        let audio_file = self
            .dao
            .get_by_id(request.id)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?
            .ok_or_else(|| QueryError::NotFound(format!("Song not found: {}", request.id)))?;

        // 解析路径（移除 protocol:// 前缀）
        let path = if let Some(idx) = audio_file.path.find("://") {
            audio_file.path[idx + 3..].to_string()
        } else {
            audio_file.path.clone()
        };

        let content_type = StreamInfo::mime_type_from_suffix(&audio_file.suffix);

        Ok(StreamInfo {
            path,
            size: audio_file.size,
            suffix: audio_file.suffix,
            bit_rate: audio_file.bit_rate,
            duration: audio_file.duration,
            content_type,
        })
    }

    /// 决定是否需要转码以及转码参数
    pub fn decide_transcoding(
        &self,
        request: &StreamRequest,
        info: &StreamInfo,
    ) -> TranscodeDecision {
        let config = self.config.as_ref();

        // 如果请求 raw 格式，不转码
        if let Some(ref format) = request.format {
            if format.eq_ignore_ascii_case("raw") {
                log::debug!(
                    "[Transcode] id={} raw format requested, skipping transcode (source: {}, {}kbps)",
                    request.id, info.suffix, info.bit_rate
                );
                return TranscodeDecision {
                    needs_transcoding: false,
                    target_format: info.suffix.clone(),
                    target_bit_rate: info.bit_rate,
                    content_type: info.content_type.clone(),
                    cache_key: generate_raw_cache_key(request.id, &info.suffix),
                    estimated_size: Some(info.size as u64),
                };
            }
        }

        // 确定目标格式
        // format=auto 或未指定时，使用服务器默认格式
        let is_auto_or_empty = request
            .format
            .as_ref()
            .map(|f| f.eq_ignore_ascii_case("auto"))
            .unwrap_or(true); // 未指定也视为 auto

        let target_format = if is_auto_or_empty {
            // 使用服务器默认格式，如果没有配置则使用源格式
            config
                .map(|c| c.default_format())
                .unwrap_or_else(|| info.suffix.clone())
        } else {
            // 使用请求指定的格式
            request.format.clone().unwrap()
        };

        // 检查源文件和目标格式的无损属性
        let source_is_lossless = config
            .map(|c| c.is_lossless(&info.suffix))
            .unwrap_or(false);
        let target_is_lossless = config
            .map(|c| c.is_lossless(&target_format))
            .unwrap_or(false);

        // 如果源是有损格式，请求无损格式，没有意义，直接返回原文件
        // （有损 -> 无损 不会提升质量，只会增加文件大小）
        if !source_is_lossless && target_is_lossless {
            log::debug!(
                "[Transcode] id={} lossy->lossless conversion is meaningless, returning raw file (source: {}, target: {})",
                request.id, info.suffix, target_format
            );
            return TranscodeDecision {
                needs_transcoding: false,
                target_format: info.suffix.clone(),
                target_bit_rate: info.bit_rate,
                content_type: info.content_type.clone(),
                cache_key: generate_raw_cache_key(request.id, &info.suffix),
                estimated_size: Some(info.size as u64),
            };
        }

        // 确定目标比特率
        // 注意：无损格式（flac, wav 等）不受比特率限制
        let target_bit_rate = if target_is_lossless {
            // 无损格式保持原始比特率，不进行比特率转换
            info.bit_rate
        } else {
            request
                .max_bit_rate
                .filter(|&br| br > 0 && br < info.bit_rate)
                .or_else(|| {
                    // 如果源是无损格式且目标是有损格式，使用默认比特率
                    config.and_then(|c| {
                        if c.is_lossless(&info.suffix) {
                            Some(c.default_bit_rate())
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or(info.bit_rate)
        };

        // 判断是否需要转码
        let format_changed = !target_format.eq_ignore_ascii_case(&info.suffix);
        // 无损格式不因比特率触发转码
        let bitrate_reduced = !target_is_lossless && target_bit_rate < info.bit_rate;
        let needs_transcoding = format_changed || bitrate_reduced;

        let content_type = if needs_transcoding {
            StreamInfo::mime_type_from_suffix(&target_format)
        } else {
            info.content_type.clone()
        };

        let cache_key = if needs_transcoding {
            generate_cache_key(request.id, &target_format, target_bit_rate)
        } else {
            generate_raw_cache_key(request.id, &info.suffix)
        };

        // 估算输出大小：bitrate (kbps) * duration (s) / 8 = bytes
        // 加上 10% 的容器开销
        let estimated_size = if needs_transcoding {
            let estimated_bytes = (target_bit_rate as u64 * info.duration as u64 * 1000 / 8) * 11 / 10;
            Some(estimated_bytes)
        } else {
            Some(info.size as u64)
        };

        log::info!(
            "[Transcode] id={} decision: needs_transcode={}, source={}@{}kbps -> target={}@{}kbps, format_changed={}, bitrate_reduced={}, estimated_size={:?}",
            request.id,
            needs_transcoding,
            info.suffix,
            info.bit_rate,
            target_format,
            target_bit_rate,
            format_changed,
            bitrate_reduced,
            estimated_size
        );

        TranscodeDecision {
            needs_transcoding,
            target_format,
            target_bit_rate,
            content_type,
            cache_key,
            estimated_size,
        }
    }

    /// 判断是否需要转码（简化版本，保持向后兼容）
    pub fn needs_transcoding(&self, request: &StreamRequest, info: &StreamInfo) -> bool {
        self.decide_transcoding(request, info).needs_transcoding
    }

    /// 获取流数据（支持缓存和转码）
    pub async fn get_stream_data(
        &self,
        request: &StreamRequest,
        info: &StreamInfo,
    ) -> Result<StreamData, QueryError> {
        let decision = self.decide_transcoding(request, info);

        // 1. 尝试从缓存获取
        if let Some(ref cache) = self.cache {
            if let Some(config) = self.config.as_ref() {
                if config.cache_enabled() {
                    if let Some(cached) = cache.get(&decision.cache_key).await {
                        log::debug!("Stream cache hit: {}", decision.cache_key);
                        return Ok(StreamData {
                            data: cached.data,
                            content_type: cached.content_type,
                            size: cached.size,
                            from_cache: true,
                        });
                    }
                }
            }
        }

        // 2. 获取数据（转码或读取原始文件）
        let (data, content_type) = if decision.needs_transcoding {
            self.transcode_file(info, &decision).await?
        } else {
            self.read_raw_file(info).await?
        };

        let size = data.len() as u64;

        // 3. 存入缓存
        if let Some(ref cache) = self.cache {
            if let Some(config) = self.config.as_ref() {
                if config.cache_enabled() {
                    let cache_data = StreamCacheData {
                        data: data.clone(),
                        content_type: content_type.clone(),
                        cache_key: decision.cache_key.clone(),
                        size,
                    };
                    cache.put(&decision.cache_key, cache_data).await;
                }
            }
        }

        Ok(StreamData {
            data,
            content_type,
            size,
            from_cache: false,
        })
    }

    /// 读取原始文件
    async fn read_raw_file(&self, info: &StreamInfo) -> Result<(Bytes, String), QueryError> {
        let mut file = File::open(&info.path)
            .await
            .map_err(|e| QueryError::ExecutionError(format!("Failed to open file: {}", e)))?;

        let mut buffer = Vec::with_capacity(info.size as usize);
        file.read_to_end(&mut buffer)
            .await
            .map_err(|e| QueryError::ExecutionError(format!("Failed to read file: {}", e)))?;

        Ok((Bytes::from(buffer), info.content_type.clone()))
    }

    /// 转码文件
    async fn transcode_file(
        &self,
        info: &StreamInfo,
        decision: &TranscodeDecision,
    ) -> Result<(Bytes, String), QueryError> {
        let transcoder = self.transcoder.as_ref().ok_or_else(|| {
            log::error!("[Transcode] Transcoder not configured!");
            QueryError::ExecutionError("Transcoder not configured".to_string())
        })?;

        log::info!(
            "[Transcode] Starting transcode: path={}, format={}, bitrate={}kbps",
            info.path,
            decision.target_format,
            decision.target_bit_rate
        );

        let start_time = std::time::Instant::now();

        // 使用 TranscodingStreamer 的 create_stream 并收集所有数据
        let mut stream = transcoder
            .create_stream(
                info.path.clone(),
                decision.target_format.clone(),
                decision.target_bit_rate,
                std::collections::HashMap::new(),
            )
            .await
            .map_err(|e: TranscodingError| {
                log::error!("[Transcode] Failed to create stream: {}", e);
                QueryError::ExecutionError(e.to_string())
            })?;

        // 收集流数据
        use futures::StreamExt;
        let mut buffer = Vec::new();
        let mut chunk_count = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e: TranscodingError| {
                log::error!("[Transcode] Error reading chunk {}: {}", chunk_count, e);
                QueryError::ExecutionError(e.to_string())
            })?;
            chunk_count += 1;
            buffer.extend_from_slice(&chunk);
        }

        let elapsed = start_time.elapsed();
        let output_size = buffer.len();
        let input_size = info.size as usize;
        let compression_ratio = if input_size > 0 {
            (output_size as f64 / input_size as f64) * 100.0
        } else {
            0.0
        };

        log::info!(
            "[Transcode] Completed: path={}, chunks={}, input_size={}, output_size={}, ratio={:.1}%, elapsed={:?}",
            info.path,
            chunk_count,
            input_size,
            output_size,
            compression_ratio,
            elapsed
        );

        Ok((Bytes::from(buffer), decision.content_type.clone()))
    }

    /// 创建转码流（流式响应，边转码边返回）
    /// 返回一个 Stream，同时在后台收集数据用于缓存
    pub async fn create_transcode_stream(
        &self,
        info: &StreamInfo,
        decision: &TranscodeDecision,
    ) -> Result<TranscodeStream, QueryError> {
        let transcoder = self.transcoder.as_ref().ok_or_else(|| {
            log::error!("[Transcode] Transcoder not configured!");
            QueryError::ExecutionError("Transcoder not configured".to_string())
        })?;

        log::info!(
            "[Transcode] Creating stream: path={}, format={}, bitrate={}kbps",
            info.path,
            decision.target_format,
            decision.target_bit_rate
        );

        let stream = transcoder
            .create_stream(
                info.path.clone(),
                decision.target_format.clone(),
                decision.target_bit_rate,
                std::collections::HashMap::new(),
            )
            .await
            .map_err(|e: TranscodingError| {
                log::error!("[Transcode] Failed to create stream: {}", e);
                QueryError::ExecutionError(e.to_string())
            })?;

        Ok(TranscodeStream {
            inner: stream,
            content_type: decision.content_type.clone(),
            cache_key: decision.cache_key.clone(),
            collected_data: Vec::new(),
            cache: self.cache.clone(),
            config: self.config.clone(),
            start_time: std::time::Instant::now(),
            chunk_count: 0,
        })
    }

    /// 检查缓存并返回缓存数据（如果有）
    pub async fn get_cached_data(&self, cache_key: &str) -> Option<StreamData> {
        if let Some(ref cache) = self.cache {
            if let Some(config) = self.config.as_ref() {
                if config.cache_enabled() {
                    if let Some(cached) = cache.get(cache_key).await {
                        log::debug!("[Stream] Cache hit: {}", cache_key);
                        return Some(StreamData {
                            data: cached.data,
                            content_type: cached.content_type,
                            size: cached.size,
                            from_cache: true,
                        });
                    }
                }
            }
        }
        None
    }
}

/// 转码流包装器，支持边转码边返回，同时收集数据用于缓存
pub struct TranscodeStream {
    inner: Box<dyn Stream<Item = Result<Vec<u8>, TranscodingError>> + Unpin + Send>,
    pub content_type: String,
    cache_key: String,
    collected_data: Vec<u8>,
    cache: Option<Arc<dyn StreamCache + Send + Sync>>,
    config: Option<Arc<dyn StreamCacheConfig + Send + Sync>>,
    start_time: std::time::Instant,
    chunk_count: usize,
}

impl Stream for TranscodeStream {
    type Item = Result<Bytes, QueryError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use futures::StreamExt;
        use std::task::Poll;

        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                self.chunk_count += 1;
                self.collected_data.extend_from_slice(&chunk);
                Poll::Ready(Some(Ok(Bytes::from(chunk))))
            }
            Poll::Ready(Some(Err(e))) => {
                log::error!("[Transcode] Stream error at chunk {}: {}", self.chunk_count, e);
                Poll::Ready(Some(Err(QueryError::ExecutionError(e.to_string()))))
            }
            Poll::Ready(None) => {
                // 流结束，记录日志
                let elapsed = self.start_time.elapsed();
                let output_size = self.collected_data.len();
                log::info!(
                    "[Transcode] Stream completed: chunks={}, output_size={}, elapsed={:?}",
                    self.chunk_count,
                    output_size,
                    elapsed
                );
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for TranscodeStream {
    fn drop(&mut self) {
        // 在流结束时，异步保存到缓存
        if !self.collected_data.is_empty() {
            if let Some(ref cache) = self.cache {
                if let Some(ref config) = self.config {
                    if config.cache_enabled() {
                        let cache = cache.clone();
                        let cache_key = self.cache_key.clone();
                        let content_type = self.content_type.clone();
                        let data = std::mem::take(&mut self.collected_data);
                        let size = data.len() as u64;

                        log::debug!("[Transcode] Caching transcoded data: key={}, size={}", cache_key, size);

                        // 使用 spawn 异步保存缓存
                        tokio::spawn(async move {
                            let cache_data = StreamCacheData {
                                data: Bytes::from(data),
                                content_type,
                                cache_key: cache_key.clone(),
                                size,
                            };
                            cache.put(&cache_key, cache_data).await;
                            log::debug!("[Transcode] Cached: {}", cache_key);
                        });
                    }
                }
            }
        }
    }
}
