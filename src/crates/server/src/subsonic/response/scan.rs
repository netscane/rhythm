use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    pub scanning: bool,

    pub count: i32,

    pub folder_count: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_scan: Option<NaiveDateTime>,
}

impl ScanStatus {
    pub fn new() -> Self {
        Self {
            scanning: false,
            count: 0,
            folder_count: 0,
            last_scan: None,
        }
    }
}
