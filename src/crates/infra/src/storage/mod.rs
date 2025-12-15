pub mod factory;
pub mod local;
pub mod smb;

pub use factory::StorageClientFactoryImpl;
pub use local::LocalStorageClient;
pub use smb::SmbStorageClient;
