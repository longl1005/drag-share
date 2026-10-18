use super::SmbPlatform;
use crate::{error::{AppError,Result},model::{ShareItem,ServiceStatus}};
pub struct Unsupported;
fn unsupported<T>()->Result<T>{Err(AppError::new("PLATFORM_NOT_SUPPORTED","当前平台正在适配中，敬请期待 P2 版本"))}
#[async_trait::async_trait]
impl SmbPlatform for Unsupported {
 async fn get_service_status(&self)->Result<ServiceStatus>{unsupported()}
 async fn enable_service(&self)->Result<()>{unsupported()}
 async fn disable_service(&self)->Result<()>{unsupported()}
 async fn list_shares(&self)->Result<Vec<ShareItem>>{unsupported()}
 async fn add_share(&self,_:&str,_:&str)->Result<()>{unsupported()}
 async fn remove_share(&self,_:&str)->Result<()>{unsupported()}
 async fn get_address(&self)->Result<(String,bool)>{unsupported()}
}
