use serde::Serialize;
#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[error("{message}")]
pub struct AppError {
    pub code: String,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, AppError>;
impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
    pub fn canceled(&self) -> bool {
        self.code == "USER_CANCELED"
    }
    pub fn io(e: impl std::fmt::Display) -> Self {
        Self::new("IO_ERROR", format!("无法完成本地操作：{e}"))
    }
}
