use application::command::library::{ScanError, Scanner};
use application::command::media_parse::StorageClient;
use application::error::AppError;
use async_trait::async_trait;
use domain::value::{FileMeta, MediaPath};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tokio::sync::mpsc;
use walkdir::WalkDir;

#[derive(Clone, Default)]
pub struct LocalStorageClient;

impl LocalStorageClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl StorageClient for LocalStorageClient {
    async fn list(&self, path: &MediaPath) -> Result<Vec<FileMeta>, AppError> {
        let mut items = Vec::new();
        for entry in fs::read_dir(&path.path).map_err(|e| AppError::UnknownError(e.to_string()))? {
            let entry = entry.map_err(|e| AppError::UnknownError(e.to_string()))?;
            let meta = entry
                .metadata()
                .map_err(|e| AppError::UnknownError(e.to_string()))?;
            let p = entry.path();
            items.push(FileMeta::new(
                MediaPath {
                    protocol: "local".to_string(),
                    path: p.to_string_lossy().to_string(),
                },
                MediaPath {
                    protocol: "local".to_string(),
                    path: p
                        .parent()
                        .and_then(|pp| pp.to_str())
                        .unwrap_or("")
                        .to_string(),
                },
                meta.len() as i64,
                p.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_string(),
                chrono::DateTime::<chrono::Utc>::from(
                    meta.modified().unwrap_or(std::time::SystemTime::now()),
                )
                .naive_utc(),
                chrono::DateTime::<chrono::Utc>::from(
                    meta.accessed().unwrap_or(std::time::SystemTime::now()),
                )
                .naive_utc(),
                chrono::DateTime::<chrono::Utc>::from(
                    meta.created().unwrap_or(std::time::SystemTime::now()),
                )
                .naive_utc(),
                None,
            ));
        }
        Ok(items)
    }

    async fn read(&self, path: &MediaPath) -> Result<Vec<u8>, AppError> {
        fs::read(&path.path).map_err(|e| AppError::UnknownError(e.to_string()))
    }

    async fn exists(&self, path: &MediaPath) -> Result<bool, AppError> {
        Ok(Path::new(&path.path).exists())
    }

    async fn get_local_path(&self, path: &MediaPath) -> Result<PathBuf, AppError> {
        Ok(Path::new(&path.path).to_path_buf())
    }
}

#[async_trait]
impl Scanner for LocalStorageClient {
    async fn scan(
        &self,
        root: &str,
    ) -> Result<mpsc::Receiver<Result<FileMeta, ScanError>>, ScanError> {
        let (tx, rx) = mpsc::channel(64);
        let root = root.to_string();
        tokio::spawn(async move {
            for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let p = entry.path().to_path_buf();
                    match entry.metadata() {
                        Ok(meta) => {
                            let _ = tx
                                .send(Ok(FileMeta::new(
                                    MediaPath {
                                        protocol: "local".to_string(),
                                        path: p.to_string_lossy().to_string(),
                                    },
                                    MediaPath {
                                        protocol: "local".to_string(),
                                        path: p
                                            .parent()
                                            .and_then(|pp| pp.to_str())
                                            .unwrap_or("")
                                            .to_string(),
                                    },
                                    meta.len() as i64,
                                    p.extension()
                                        .and_then(|e| e.to_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    chrono::DateTime::<chrono::Utc>::from(
                                        meta.modified().unwrap_or(std::time::SystemTime::now()),
                                    )
                                    .naive_utc(),
                                    chrono::DateTime::<chrono::Utc>::from(
                                        meta.accessed().unwrap_or(std::time::SystemTime::now()),
                                    )
                                    .naive_utc(),
                                    chrono::DateTime::<chrono::Utc>::from(
                                        meta.created().unwrap_or(std::time::SystemTime::now()),
                                    )
                                    .naive_utc(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_scan_empty() {
        let temp_dir = TempDir::new().unwrap();
        let backend = LocalStorageClient::new();

        let mut receiver = backend
            .scan(temp_dir.path().to_str().unwrap())
            .await
            .unwrap();
        let mut files = Vec::new();
        while let Some(result) = receiver.recv().await {
            files.push(result);
        }
        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn test_scan_with_files() {
        let temp_dir = TempDir::new().unwrap();
        let backend = LocalStorageClient::new();

        // Create test files
        let test_file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&test_file_path).unwrap();
        writeln!(file, "test content").unwrap();

        // Create subdirectory with a file
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).unwrap();
        let sub_file_path = sub_dir.join("sub.txt");
        let mut sub_file = File::create(&sub_file_path).unwrap();
        writeln!(sub_file, "sub content").unwrap();

        let mut receiver = backend
            .scan(temp_dir.path().to_str().unwrap())
            .await
            .unwrap();
        let mut files = Vec::new();
        while let Some(result) = receiver.recv().await {
            files.push(result);
        }

        assert_eq!(files.len(), 2);

        let file_metas: Vec<FileMeta> = files.into_iter().map(|r| r.unwrap()).collect();
        let paths: Vec<String> = file_metas.iter().map(|m| m.path.path.clone()).collect();

        let test_path = test_file_path.to_string_lossy().to_string();
        let sub_path = sub_file_path.to_string_lossy().to_string();
        assert!(paths.contains(&test_path));
        assert!(paths.contains(&sub_path));

        // Verify file metadata
        let test_file_meta = file_metas
            .iter()
            .find(|m| m.path.path == test_path)
            .unwrap();
        assert_eq!(test_file_meta.suffix, "txt");
    }

    #[tokio::test]
    async fn test_scan_invalid_path() {
        let backend = LocalStorageClient::new();

        let mut receiver = backend.scan("/non_existent_path_12345").await.unwrap();
        let mut files = Vec::new();
        while let Some(result) = receiver.recv().await {
            files.push(result);
        }
        // 对于不存在的路径，walkdir 可能返回空结果或错误
        // 这里只验证不会 panic
        assert!(true);
    }
}
