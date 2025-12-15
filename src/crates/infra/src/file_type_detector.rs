use application::command::library::FileTypeDetector;
use domain::value::FileType;

/// Default implementation of FileTypeDetector that detects file types based on file extensions
pub struct DefaultFileTypeDetector;

impl DefaultFileTypeDetector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultFileTypeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl FileTypeDetector for DefaultFileTypeDetector {
    fn detect(&self, suffix: &str) -> FileType {
        let suffix_lower = suffix.to_lowercase();

        match suffix_lower.as_str() {
            // Audio file extensions
            "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" | "opus" | "aiff" | "au"
            | "ra" => FileType::Audio,
            // Image file extensions
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "webp" | "svg" | "ico"
            | "psd" | "raw" | "cr2" | "nef" | "arw" | "dng" => FileType::Image,
            // NFO file extension
            "nfo" => FileType::Nfo,
            // Default to Other for unknown extensions
            _ => FileType::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_detection() {
        let detector = DefaultFileTypeDetector::new();

        assert_eq!(detector.detect("mp3"), FileType::Audio);
        assert_eq!(detector.detect("MP3"), FileType::Audio);
        assert_eq!(detector.detect("flac"), FileType::Audio);
        assert_eq!(detector.detect("wav"), FileType::Audio);
        assert_eq!(detector.detect("aac"), FileType::Audio);
        assert_eq!(detector.detect("ogg"), FileType::Audio);
    }

    #[test]
    fn test_image_detection() {
        let detector = DefaultFileTypeDetector::new();

        assert_eq!(detector.detect("jpg"), FileType::Image);
        assert_eq!(detector.detect("JPG"), FileType::Image);
        assert_eq!(detector.detect("jpeg"), FileType::Image);
        assert_eq!(detector.detect("png"), FileType::Image);
        assert_eq!(detector.detect("gif"), FileType::Image);
        assert_eq!(detector.detect("bmp"), FileType::Image);
    }

    #[test]
    fn test_nfo_detection() {
        let detector = DefaultFileTypeDetector::new();

        assert_eq!(detector.detect("nfo"), FileType::Nfo);
        assert_eq!(detector.detect("NFO"), FileType::Nfo);
    }

    #[test]
    fn test_other_detection() {
        let detector = DefaultFileTypeDetector::new();

        assert_eq!(detector.detect("txt"), FileType::Other);
        assert_eq!(detector.detect("pdf"), FileType::Other);
        assert_eq!(detector.detect("doc"), FileType::Other);
        assert_eq!(detector.detect(""), FileType::Other);
    }
}
