#[cfg(target_os = "macos")]
mod macos;
pub mod parser;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod unsupported;
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
#[cfg(any(target_os = "windows", test))]
mod windows;
use crate::{
    error::Result,
    model::{ServiceStatus, ShareItem},
};
#[async_trait::async_trait]
pub trait SmbPlatform: Send + Sync {
    async fn get_service_status(&self) -> Result<ServiceStatus>;
    async fn enable_service(&self) -> Result<()>;
    async fn disable_service(&self) -> Result<()>;
    async fn list_shares(&self) -> Result<Vec<ShareItem>>;
    async fn add_share(&self, path: &str, name: &str) -> Result<()>;
    async fn remove_share(&self, record_name: &str) -> Result<()>;
    async fn get_address(&self) -> Result<(String, bool)>;
}
pub fn native() -> Box<dyn SmbPlatform> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOs)
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::Windows)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Box::new(unsupported::Unsupported)
    }
}
