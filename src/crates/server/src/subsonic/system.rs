use crate::subsonic::response::Subsonic;
use log::info;

/// ping - 测试服务器连接
///
/// 根据 Subsonic/OpenSubsonic 规范 (Since 1.0.0):
/// - 用于测试与服务器的连接状态
/// - 返回空的 subsonic-response（表示连接正常）
///
/// OpenSubsonic 扩展:
/// - 返回 type（服务器名称）
/// - 返回 serverVersion（服务器版本）
/// - 返回 openSubsonic: true
pub async fn ping() -> Subsonic {
    info!("ping");
    Subsonic::default()
}
