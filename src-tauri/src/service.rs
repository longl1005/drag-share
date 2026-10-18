use crate::{audit,error::{AppError,Result},model::{Snapshot,ServiceStatus},platform::SmbPlatform,security};
use std::sync::atomic::{AtomicBool,Ordering};
pub struct Service { pub platform:Box<dyn SmbPlatform>, pub mutation:tokio::sync::Mutex<()>, pub native_dialog:AtomicBool }
impl Service {
    pub fn new(platform:Box<dyn SmbPlatform>)->Self {Self{platform,mutation:tokio::sync::Mutex::new(()),native_dialog:AtomicBool::new(false)}}
    pub fn busy(&self)->bool {self.native_dialog.load(Ordering::SeqCst)}
    pub async fn snapshot(&self)->Snapshot {
        let (status,shares,address)=tokio::join!(self.platform.get_service_status(),self.platform.list_shares(),self.platform.get_address());
        let mut warnings=vec![];
        let status=status.unwrap_or_else(|e|{warnings.push(e.message);ServiceStatus::Unknown});
        let shares=shares.unwrap_or_else(|e|{warnings.push(e.message);vec![]});
        let (address,address_fallback)=address.unwrap_or_else(|e|{warnings.push(e.message);(String::new(),true)});
        Snapshot{status,shares,address,address_fallback,warnings}
    }
    pub async fn set_enabled(&self,enabled:bool)->Result<()> {
        let _lock=self.mutation.try_lock().map_err(|_|AppError::new("BUSY","操作正在进行中"))?;
        let start=std::time::Instant::now();
        let result=if enabled{self.platform.enable_service().await}else{self.platform.disable_service().await};
        audit::record(if enabled{"enable_service"}else{"disable_service"},None,result.as_ref().err(),start.elapsed().as_millis());result
    }
    pub async fn add(&self,path:&str)->Result<()> {
        let _lock=self.mutation.try_lock().map_err(|_|AppError::new("BUSY","操作正在进行中"))?;
        let start=std::time::Instant::now();
        let result=self.add_inner(path).await;
        audit::record("add_share",Some(path),result.as_ref().err(),start.elapsed().as_millis());result
    }
    async fn add_inner(&self,path:&str)->Result<()> {
        self.require_enabled().await?;
        let path=security::canonical_directory(path)?;
        let shares=self.platform.list_shares().await?;
        if shares.iter().any(|s|security::canonical_directory(&s.path).unwrap_or_else(|_|s.path.clone())==path){return Err(AppError::new("DUPLICATE","该目录已在共享列表中"));}
        let base=std::path::Path::new(&path).file_name().and_then(|s|s.to_str()).ok_or_else(||AppError::new("INVALID_PATH","请选择一个具体文件夹，不支持共享磁盘根目录"))?;
        let mut name=base.to_owned();let mut i=2;
        while shares.iter().any(|s|s.id==name || s.name==name){name=format!("{base} ({i})");i+=1;}
        self.platform.add_share(&path,&name).await?;
        if !self.platform.list_shares().await?.iter().any(|s|s.id==name && s.path==path){return Err(AppError::new("VERIFY_FAILED","无法确认目录已共享，请刷新列表检查"));}Ok(())
    }
    pub async fn remove(&self,id:&str)->Result<()> {
        let _lock=self.mutation.try_lock().map_err(|_|AppError::new("BUSY","操作正在进行中"))?;
        let start=std::time::Instant::now();
        let result=async {
            self.require_enabled().await?;
            if !self.platform.list_shares().await?.iter().any(|s|s.id==id){return Err(AppError::new("NOT_FOUND","该共享已不存在，请刷新列表"));}
            self.platform.remove_share(id).await?;
            if self.platform.list_shares().await?.iter().any(|s|s.id==id){return Err(AppError::new("VERIFY_FAILED","无法确认共享已移除，请刷新检查"));}Ok(())
        }.await;
        audit::record("remove_share",Some(id),result.as_ref().err(),start.elapsed().as_millis());result
    }
    async fn require_enabled(&self)->Result<()> {if self.platform.get_service_status().await?!=ServiceStatus::Enabled{return Err(AppError::new("SERVICE_DISABLED","请先开启共享服务"));}Ok(())}
}
