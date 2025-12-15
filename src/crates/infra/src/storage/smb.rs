use application::command::library::{ScanError, Scanner};
use application::command::media_parse::StorageClient;
use application::error::AppError;
use domain::value::{FileMeta, MediaPath};
use pavao::{SmbClient, SmbCredentials, SmbDirentType, SmbOpenOptions, SmbOptions};
use std::collections::VecDeque;
use std::env;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::sync::mpsc;

#[derive(Clone, Default)]
pub struct SmbStorageClient;

impl SmbStorageClient {
    pub fn new() -> Self {
        Self
    }

    fn parse_smb_url(&self, url: &str) -> Result<(String, String, String), AppError> {
        let without_scheme = url.strip_prefix("smb://").ok_or_else(|| {
            AppError::UnknownError("Invalid SMB URL, must start with smb://".to_string())
        })?;
        let mut parts = without_scheme.splitn(3, '/');
        let server = parts
            .next()
            .ok_or_else(|| AppError::UnknownError("Missing server in SMB URL".to_string()))?
            .to_string();
        let share = parts
            .next()
            .ok_or_else(|| AppError::UnknownError("Missing share in SMB URL".to_string()))?
            .to_string();
        let remote_path = parts.next().unwrap_or("").to_string();
        Ok((server, share, remote_path))
    }

    fn create_client(&self, server: &str, share: &str) -> Result<SmbClient, AppError> {
        let username = env::var("SMB_USERNAME").unwrap_or_default();
        let password = env::var("SMB_PASSWORD").unwrap_or_default();
        SmbClient::new(
            SmbCredentials::default()
                .server(server)
                .share(share)
                .username(&username)
                .password(&password),
            SmbOptions::default(),
        )
        .map_err(|e| AppError::UnknownError(format!("Failed to create SMB client: {}", e)))
    }

    fn share_path(&self, share: &str, remote_path: &str) -> String {
        if remote_path.is_empty() {
            format!("/{}", share)
        } else {
            format!("/{}/{}", share, remote_path)
        }
    }
}

#[async_trait::async_trait]
impl StorageClient for SmbStorageClient {
    async fn list(&self, path: &MediaPath) -> Result<Vec<FileMeta>, AppError> {
        let protocol = path.protocol.clone();
        let path = path.path.clone();
        let (server, share, remote_path) = self.parse_smb_url(&path)?;
        let client = self.create_client(&server, &share)?;
        let dir_path = self.share_path(&share, &remote_path);
        let entries = client
            .list_dir(&dir_path)
            .map_err(|e| AppError::UnknownError(e.to_string()))?;

        let mut items = Vec::new();
        for entry in entries {
            let name = entry.name();
            let child_remote = if remote_path.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", remote_path, name)
            };
            let full_path = self.share_path(&share, &child_remote);
            let stat = client
                .stat(&full_path)
                .map_err(|e| AppError::UnknownError(e.to_string()))?;
            let full_url = format!("smb://{}/{}/{}", server, share, child_remote);
            let parent_url = format!("smb://{}/{}/{}", server, share, remote_path);
            items.push(FileMeta::new(
                MediaPath {
                    protocol: protocol.clone(),
                    path: full_url,
                },
                MediaPath {
                    protocol: protocol.clone(),
                    path: parent_url,
                },
                stat.size as i64,
                Path::new(name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_string(),
                chrono::DateTime::<chrono::Utc>::from(stat.modified).naive_utc(),
                chrono::DateTime::<chrono::Utc>::from(stat.modified).naive_utc(),
                chrono::DateTime::<chrono::Utc>::from(stat.created).naive_utc(),
                None,
            ));
        }
        Ok(items)
    }

    async fn read(&self, path: &MediaPath) -> Result<Vec<u8>, AppError> {
        let protocol = path.protocol.clone();
        let path = path.path.clone();
        let (server, share, remote_path) = self.parse_smb_url(&path)?;
        let client = self.create_client(&server, &share)?;
        let full_path = self.share_path(&share, &remote_path);
        let mut file = client
            .open_with(&full_path, SmbOpenOptions::default().read(true))
            .map_err(|e| AppError::UnknownError(e.to_string()))?;
        use std::io::Read;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| AppError::UnknownError(e.to_string()))?;
        Ok(buf)
    }

    async fn exists(&self, path: &MediaPath) -> Result<bool, AppError> {
        let path = path.path.clone();
        let (server, share, remote_path) = self.parse_smb_url(&path)?;
        let client = self.create_client(&server, &share)?;
        let full_path = self.share_path(&share, &remote_path);
        Ok(client.stat(&full_path).is_ok())
    }

    async fn get_local_path(&self, path: &MediaPath) -> Result<PathBuf, AppError> {
        let bytes = self.read(path).await?;
        let mut tmp = NamedTempFile::new().map_err(|e| {
            AppError::ParseAudioMetadataError(format!("Failed to create temp file: {:?}", e))
        })?;
        tmp.as_file_mut().write_all(&bytes).map_err(|e| {
            AppError::ParseAudioMetadataError(format!("Failed to write temp file: {:?}", e))
        })?;
        Ok(tmp.path().to_path_buf())
    }
}

#[async_trait::async_trait]
impl Scanner for SmbStorageClient {
    async fn scan(
        &self,
        root: &str,
    ) -> Result<mpsc::Receiver<Result<FileMeta, ScanError>>, ScanError> {
        let protocol = "smb".to_string();
        let path = root.to_string();
        let (server, share, remote_path) = self
            .parse_smb_url(&path)
            .map_err(|e| ScanError::OtherError(e.to_string()))?;
        let client = self
            .create_client(&server, &share)
            .map_err(|e| ScanError::OtherError(e.to_string()))?;
        let (tx, rx) = mpsc::channel(64);

        let share_clone = share.clone();
        tokio::spawn(async move {
            let mut queue: VecDeque<String> = VecDeque::new();
            queue.push_back(remote_path.clone());

            while let Some(current) = queue.pop_front() {
                let full_dir = if current.is_empty() {
                    format!("/{}", share_clone)
                } else {
                    format!("/{}/{}", share_clone, current)
                };

                let entries = match client.list_dir(&full_dir) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                for entry in entries {
                    let name = entry.name();
                    let child_remote = if current.is_empty() {
                        name.to_string()
                    } else {
                        format!("{}/{}", current, name)
                    };
                    let child_full = format!("/{}/{}", share_clone, child_remote);

                    if entry.get_type() == SmbDirentType::Dir {
                        queue.push_back(child_remote);
                        continue;
                    }

                    match client.stat(&child_full) {
                        Ok(stat) => {
                            let full_url =
                                format!("smb://{}/{}/{}", server, share_clone, child_remote);
                            let parent_url =
                                format!("smb://{}/{}/{}", server, share_clone, current);
                            let _ = tx
                                .send(Ok(FileMeta::new(
                                    MediaPath {
                                        protocol: protocol.clone(),
                                        path: full_url,
                                    },
                                    MediaPath {
                                        protocol: protocol.clone(),
                                        path: parent_url,
                                    },
                                    stat.size as i64,
                                    Path::new(name)
                                        .extension()
                                        .and_then(|e| e.to_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    chrono::DateTime::<chrono::Utc>::from(stat.modified)
                                        .naive_utc(),
                                    chrono::DateTime::<chrono::Utc>::from(stat.modified)
                                        .naive_utc(),
                                    chrono::DateTime::<chrono::Utc>::from(stat.created).naive_utc(),
                                    None,
                                )))
                                .await;
                        }
                        Err(_) => {}
                    }
                }
            }
        });

        Ok(rx)
    }
}
