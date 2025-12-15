use domain::cover_art::CoverSourceType;
use domain::value::{AudioMetadata, FileMeta, LibraryId};

pub struct AudioFileParsed {
    pub library_id: LibraryId,
    pub metadata: AudioMetadata,
    pub file_info: FileMeta,
}

pub struct ImageFileParsed {
    pub library_id: LibraryId,
    pub file_info: FileMeta,
    pub source: CoverSourceType,
}

pub enum AppEvent {
    AudioFileParsed(AudioFileParsed),
    ImageFileParsed(ImageFileParsed),
}
