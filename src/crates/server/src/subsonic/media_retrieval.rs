use crate::subsonic::response::error::SubsonicError;
use crate::subsonic::response::Subsonic;
use crate::AppState;
use actix_web::{http::header, web, HttpRequest, HttpResponse, Responder};
use application::query::config::CoverArtConfig;
use application::query::dao::{AudioFileDao, CoverArtDao};
use application::query::get_cover_art::{CoverArtCache, CoverArtReader, GetCoverArt};
use application::query::stream_cache::{StreamCache, StreamCacheConfig};
use application::query::stream_media::{StreamMedia, StreamRequest, TranscodeStream};
use domain::transcoding::TranscodingStreamer;
use futures::StreamExt;
use infra::auth::AuthConfig;
use infra::config::TranscodingConfig;
use infra::repository::postgres::query::audio_file::AudioFileDaoImpl;
use infra::repository::postgres::query::cover_art::CoverArtDaoImpl;
use infra::{CoverArtCacheImpl, CoverArtReaderImpl};
use serde::Deserialize;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};

/// getCoverArt API 请求参数
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCoverArtQuery {
    pub id: String,
    #[serde(default)]
    pub size: Option<u32>,
}

/// getCoverArt - 获取封面艺术图片
pub async fn get_cover_art(
    state: web::Data<AppState>,
    query: web::Query<GetCoverArtQuery>,
) -> HttpResponse {
    let cover_art_dao: Arc<dyn CoverArtDao + Send + Sync> =
        Arc::new(CoverArtDaoImpl::new(state.db.clone()));
    let cover_art_config: Arc<dyn CoverArtConfig + Send + Sync> =
        Arc::new(state.app_cfg.clone());
    let cover_art_reader: Arc<dyn CoverArtReader + Send + Sync> =
        Arc::new(CoverArtReaderImpl::new());
    let cover_art_cache: Arc<dyn CoverArtCache + Send + Sync> = state.cover_art_cache.clone();
    let usecase = GetCoverArt::new(
        cover_art_dao,
        cover_art_config,
        cover_art_reader,
        cover_art_cache,
    );

    let token_service = infra::auth::JwtTokenService::new(
        &state.app_cfg.jwt_secret(),
        state.app_cfg.jwt_expire_secs(),
    );

    let cover_art_id = parse_cover_art_id(&query.id, &token_service);

    match usecase.get_or_placeholder(&cover_art_id, query.size).await {
        Ok(cover_data) => HttpResponse::Ok()
            .insert_header((header::CONTENT_TYPE, cover_data.mime_type))
            .insert_header((header::CACHE_CONTROL, "public, max-age=315360000"))
            .insert_header((header::ETAG, format!("\"{}\"", cover_data.cache_key)))
            .body(cover_data.data),
        Err(e) => {
            log::warn!("Failed to get cover art for {}: {}", query.id, e);
            HttpResponse::NotFound().finish()
        }
    }
}

fn parse_cover_art_id(id: &str, token_service: &infra::auth::JwtTokenService) -> String {
    if id.contains('.') {
        match token_service.verify_cover_art_token(id) {
            Ok(cover_art_id) => cover_art_id,
            Err(e) => {
                log::debug!("Token verification failed, using raw id: {}", e);
                id.to_string()
            }
        }
    } else {
        id.to_string()
    }
}

/// stream API 请求参数
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamQuery {
    pub id: i64,
    #[serde(default)]
    pub max_bit_rate: Option<i32>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub time_offset: Option<i32>,
    #[serde(default)]
    pub estimate_content_length: Option<bool>,
}

/// TranscodingConfig 的 StreamCacheConfig 适配器
struct TranscodingConfigAdapter {
    config: TranscodingConfig,
}

impl TranscodingConfigAdapter {
    fn new(config: TranscodingConfig) -> Self {
        Self { config }
    }
}

impl StreamCacheConfig for TranscodingConfigAdapter {
    fn cache_enabled(&self) -> bool {
        self.config.cache_enabled
    }

    fn default_format(&self) -> String {
        self.config.default_format.clone()
    }

    fn default_bit_rate(&self) -> i32 {
        self.config.default_bit_rate
    }

    fn is_lossless(&self, format: &str) -> bool {
        self.config.is_lossless(format)
    }
}

/// stream - 流式传输媒体文件
pub async fn stream(
    state: web::Data<AppState>,
    query: web::Query<StreamQuery>,
    req: HttpRequest,
) -> StreamResponse {
    log::info!(
        "[Stream] Request: id={}, format={:?}, max_bit_rate={:?}, time_offset={:?}",
        query.id,
        query.format,
        query.max_bit_rate,
        query.time_offset
    );

    let audio_file_dao: Arc<dyn AudioFileDao + Send + Sync> =
        Arc::new(AudioFileDaoImpl::new(state.db.clone()));

    // 创建配置适配器
    let transcoding_cfg = state.app_cfg.transcoding();
    let config_adapter: Arc<dyn StreamCacheConfig + Send + Sync> =
        Arc::new(TranscodingConfigAdapter::new(transcoding_cfg));

    // 创建缓存和转码器引用
    let stream_cache: Arc<dyn StreamCache + Send + Sync> = state.stream_cache.clone();
    let transcoder: Arc<dyn TranscodingStreamer + Send + Sync> = state.transcoder.clone();

    // 构建 usecase
    let usecase = StreamMedia::new(audio_file_dao)
        .with_cache(stream_cache)
        .with_config(config_adapter)
        .with_transcoder(transcoder);

    let request = StreamRequest {
        id: query.id,
        max_bit_rate: query.max_bit_rate,
        format: query.format.clone(),
        time_offset: query.time_offset,
        estimate_content_length: query.estimate_content_length.unwrap_or(false),
    };

    // 获取流媒体信息
    let stream_info = match usecase.get_stream_info(&request).await {
        Ok(info) => {
            log::debug!(
                "[Stream] File info: id={}, path={}, suffix={}, bitrate={}kbps, size={}",
                query.id, info.path, info.suffix, info.bit_rate, info.size
            );
            info
        }
        Err(e) => {
            log::warn!("[Stream] Failed to get stream info for {}: {}", query.id, e);
            return StreamResponse::Error(
                SubsonicError::error_data_not_found().wrap(format!("Song not found: {}", query.id)),
            );
        }
    };

    // 决定是否转码
    let decision = usecase.decide_transcoding(&request, &stream_info);

    // 检查是否有 Range 请求
    let range_header = req
        .headers()
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok());

    log::debug!(
        "[Stream] id={}, needs_transcoding={}, range={:?}",
        query.id, decision.needs_transcoding, range_header
    );

    // 如果需要转码或没有 Range 请求，使用流处理逻辑
    if decision.needs_transcoding || range_header.is_none() {
        // 1. 先检查缓存
        if let Some(cached) = usecase.get_cached_data(&decision.cache_key).await {
            log::info!(
                "[Stream] Cache hit: id={}, size={}, content_type={}",
                query.id,
                cached.size,
                cached.content_type
            );

            // 如果有 Range 请求，处理部分内容
            if let Some(range_str) = range_header {
                return StreamResponse::Binary(handle_range_request_from_bytes(
                    &cached.data,
                    range_str,
                    &cached.content_type,
                ));
            }

            return StreamResponse::Binary(
                HttpResponse::Ok()
                    .insert_header((header::CONTENT_TYPE, cached.content_type))
                    .insert_header((header::CONTENT_LENGTH, cached.size))
                    .insert_header((header::ACCEPT_RANGES, "bytes"))
                    .body(cached.data),
            );
        }

        // 2. 需要转码：使用流式响应
        if decision.needs_transcoding {
            // 如果有 Range 请求但没有缓存，需要先完成转码才能支持 Range
            // 这里选择完成转码后再处理 Range 请求
            if range_header.is_some() {
                log::info!(
                    "[Stream] Transcoding with Range request, completing transcode first: id={}, format={}, bitrate={}kbps",
                    query.id,
                    decision.target_format,
                    decision.target_bit_rate
                );
                
                // 完成转码获取完整数据
                match usecase.get_stream_data(&request, &stream_info).await {
                    Ok(stream_data) => {
                        log::info!(
                            "[Stream] Transcode completed for Range request: id={}, size={}, content_type={}",
                            query.id,
                            stream_data.size,
                            stream_data.content_type
                        );
                        
                        return StreamResponse::Binary(handle_range_request_from_bytes(
                            &stream_data.data,
                            range_header.unwrap(),
                            &stream_data.content_type,
                        ));
                    }
                    Err(e) => {
                        log::error!("[Stream] Failed to transcode for Range request: {}", e);
                        return StreamResponse::Error(
                            SubsonicError::error_generic().wrap(format!("Failed to transcode: {}", e)),
                        );
                    }
                }
            }
            
            log::info!(
                "[Stream] Starting streaming transcode: id={}, format={}, bitrate={}kbps, estimated_size={:?}, estimate_content_length={}",
                query.id,
                decision.target_format,
                decision.target_bit_rate,
                decision.estimated_size,
                request.estimate_content_length
            );

            match usecase.create_transcode_stream(&stream_info, &decision).await {
                Ok(transcode_stream) => {
                    let content_type = transcode_stream.content_type.clone();

                    // 使用 actix-web 的流式响应
                    let body_stream = transcode_stream.map(|result| {
                        result.map_err(|e| {
                            actix_web::error::ErrorInternalServerError(e.to_string())
                        })
                    });

                    let mut response = HttpResponse::Ok();
                    response.insert_header((header::CONTENT_TYPE, content_type));
                    
                    // 根据 estimateContentLength 参数决定是否设置 Content-Length
                    // (Since 1.8.0) 如果设置为 true，则为转码媒体设置估算的 Content-Length
                    if request.estimate_content_length {
                        if let Some(estimated_size) = decision.estimated_size {
                            response.insert_header((header::CONTENT_LENGTH, estimated_size));
                            // 注意：流式转码不支持真正的 Range 请求，不设置 Accept-Ranges
                        }
                    }

                    return StreamResponse::Binary(response.streaming(body_stream));
                }
                Err(e) => {
                    log::error!("[Stream] Failed to create transcode stream: {}", e);
                    return StreamResponse::Error(
                        SubsonicError::error_generic().wrap(format!("Failed to transcode: {}", e)),
                    );
                }
            }
        }

        // 3. 不需要转码：读取原始文件
        match usecase.get_stream_data(&request, &stream_info).await {
            Ok(stream_data) => {
                log::info!(
                    "[Stream] Success: id={}, cached={}, size={}, content_type={}",
                    query.id,
                    stream_data.from_cache,
                    stream_data.size,
                    stream_data.content_type
                );

                // 如果有 Range 请求，处理部分内容
                if let Some(range_str) = range_header {
                    return StreamResponse::Binary(handle_range_request_from_bytes(
                        &stream_data.data,
                        range_str,
                        &stream_data.content_type,
                    ));
                }

                StreamResponse::Binary(
                    HttpResponse::Ok()
                        .insert_header((header::CONTENT_TYPE, stream_data.content_type))
                        .insert_header((header::CONTENT_LENGTH, stream_data.size))
                        .insert_header((header::ACCEPT_RANGES, "bytes"))
                        .body(stream_data.data),
                )
            }
            Err(e) => {
                log::error!("[Stream] Failed to get stream data for {}: {}", query.id, e);
                StreamResponse::Error(
                    SubsonicError::error_generic().wrap(format!("Failed to stream: {}", e)),
                )
            }
        }
    } else {
        // 原始文件 + Range 请求：直接从文件系统读取以支持 seek
        log::debug!("[Stream] id={}, serving raw file with range support", query.id);
        let file = match File::open(&stream_info.path).await {
            Ok(f) => f,
            Err(e) => {
                log::error!("[Stream] Failed to open file {}: {}", stream_info.path, e);
                return StreamResponse::Error(
                    SubsonicError::error_generic().wrap(format!("Failed to open file: {}", e)),
                );
            }
        };

        let metadata = match file.metadata().await {
            Ok(m) => m,
            Err(e) => {
                log::error!("[Stream] Failed to get file metadata: {}", e);
                return StreamResponse::Error(
                    SubsonicError::error_generic()
                        .wrap(format!("Failed to get file metadata: {}", e)),
                );
            }
        };
        let file_size = metadata.len();

        StreamResponse::Binary(
            handle_range_request(file, file_size, range_header.unwrap(), &stream_info.content_type)
                .await,
        )
    }
}

/// Stream 响应类型
pub enum StreamResponse {
    Binary(HttpResponse),
    Error(SubsonicError),
}

impl Responder for StreamResponse {
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        match self {
            StreamResponse::Binary(response) => response,
            StreamResponse::Error(error) => {
                let subsonic: Subsonic = error.into();
                subsonic.respond_to(req)
            }
        }
    }
}

/// 从内存数据处理 Range 请求
fn handle_range_request_from_bytes(data: &[u8], range_str: &str, content_type: &str) -> HttpResponse {
    let file_size = data.len() as u64;
    let range = parse_range(range_str, file_size);

    let (start, end) = match range {
        Some((s, e)) => (s, e),
        None => {
            return HttpResponse::RangeNotSatisfiable()
                .insert_header((header::CONTENT_RANGE, format!("bytes */{}", file_size)))
                .finish();
        }
    };

    let length = end - start + 1;
    let slice = &data[start as usize..=end as usize];

    HttpResponse::PartialContent()
        .insert_header((header::CONTENT_TYPE, content_type.to_string()))
        .insert_header((header::CONTENT_LENGTH, length))
        .insert_header((
            header::CONTENT_RANGE,
            format!("bytes {}-{}/{}", start, end, file_size),
        ))
        .insert_header((header::ACCEPT_RANGES, "bytes"))
        .body(slice.to_vec())
}

/// 处理 Range 请求（从文件）
async fn handle_range_request(
    mut file: File,
    file_size: u64,
    range_str: &str,
    content_type: &str,
) -> HttpResponse {
    let range = parse_range(range_str, file_size);

    let (start, end) = match range {
        Some((s, e)) => (s, e),
        None => {
            return HttpResponse::RangeNotSatisfiable()
                .insert_header((header::CONTENT_RANGE, format!("bytes */{}", file_size)))
                .finish();
        }
    };

    let length = end - start + 1;

    if let Err(e) = file.seek(SeekFrom::Start(start)).await {
        log::error!("Failed to seek file: {}", e);
        return HttpResponse::InternalServerError().finish();
    }

    let mut buffer = vec![0u8; length as usize];
    if let Err(e) = file.read_exact(&mut buffer).await {
        log::error!("Failed to read file range: {}", e);
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::PartialContent()
        .insert_header((header::CONTENT_TYPE, content_type.to_string()))
        .insert_header((header::CONTENT_LENGTH, length))
        .insert_header((
            header::CONTENT_RANGE,
            format!("bytes {}-{}/{}", start, end, file_size),
        ))
        .insert_header((header::ACCEPT_RANGES, "bytes"))
        .body(buffer)
}

/// 解析 Range 请求头
fn parse_range(range_str: &str, file_size: u64) -> Option<(u64, u64)> {
    let range_str = range_str.strip_prefix("bytes=")?;

    if let Some(suffix_len) = range_str.strip_prefix('-') {
        let suffix: u64 = suffix_len.parse().ok()?;
        if suffix > file_size {
            return Some((0, file_size - 1));
        }
        return Some((file_size - suffix, file_size - 1));
    }

    let parts: Vec<&str> = range_str.split('-').collect();
    if parts.len() != 2 {
        return None;
    }

    let start: u64 = parts[0].parse().ok()?;
    if start >= file_size {
        return None;
    }

    let end = if parts[1].is_empty() {
        file_size - 1
    } else {
        let end: u64 = parts[1].parse().ok()?;
        end.min(file_size - 1)
    };

    if start > end {
        return None;
    }

    Some((start, end))
}
