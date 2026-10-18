use crate::{
    audit,
    error::{AppError, Result},
    model::{ServiceStatus, Snapshot},
    platform::SmbPlatform,
    security,
};
use std::sync::atomic::{AtomicBool, Ordering};
pub struct Service {
    pub platform: Box<dyn SmbPlatform>,
    pub mutation: tokio::sync::Mutex<()>,
    pub native_dialog: AtomicBool,
}
impl Service {
    pub fn new(platform: Box<dyn SmbPlatform>) -> Self {
        Self {
            platform,
            mutation: tokio::sync::Mutex::new(()),
            native_dialog: AtomicBool::new(false),
        }
    }
    pub fn busy(&self) -> bool {
        self.native_dialog.load(Ordering::SeqCst)
    }
    pub async fn snapshot(&self) -> Snapshot {
        let (status, shares, address) = tokio::join!(
            self.platform.get_service_status(),
            self.platform.list_shares(),
            self.platform.get_address()
        );
        let mut warnings = vec![];
        let status = status.unwrap_or_else(|e| {
            warnings.push(e.message);
            ServiceStatus::Unknown
        });
        let shares = shares.unwrap_or_else(|e| {
            warnings.push(e.message);
            vec![]
        });
        let (address, address_fallback) = address.unwrap_or_else(|e| {
            warnings.push(e.message);
            (String::new(), true)
        });
        Snapshot {
            status,
            shares,
            address,
            address_fallback,
            warnings,
        }
    }
    pub async fn set_enabled(&self, enabled: bool) -> Result<()> {
        let _lock = self
            .mutation
            .try_lock()
            .map_err(|_| AppError::new("BUSY", "操作正在进行中"))?;
        let start = std::time::Instant::now();
        let result = if enabled {
            self.platform.enable_service().await
        } else {
            self.platform.disable_service().await
        };
        audit::record(
            if enabled {
                "enable_service"
            } else {
                "disable_service"
            },
            None,
            result.as_ref().err(),
            start.elapsed().as_millis(),
        );
        result
    }
    pub async fn add(&self, path: &str) -> Result<()> {
        let _lock = self
            .mutation
            .try_lock()
            .map_err(|_| AppError::new("BUSY", "操作正在进行中"))?;
        let start = std::time::Instant::now();
        let result = self.add_inner(path).await;
        audit::record(
            "add_share",
            Some(path),
            result.as_ref().err(),
            start.elapsed().as_millis(),
        );
        result
    }
    async fn add_inner(&self, path: &str) -> Result<()> {
        self.require_enabled().await?;
        let path = security::canonical_directory(path)?;
        let shares = self.platform.list_shares().await?;
        if shares.iter().any(|s| {
            security::canonical_directory(&s.path).unwrap_or_else(|_| s.path.clone()) == path
        }) {
            return Err(AppError::new("DUPLICATE", "该目录已在共享列表中"));
        }
        let base = std::path::Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                AppError::new("INVALID_PATH", "请选择一个具体文件夹，不支持共享磁盘根目录")
            })?;
        let mut name = base.to_owned();
        let mut i = 2;
        while shares.iter().any(|s| s.id == name || s.name == name) {
            name = format!("{base} ({i})");
            i += 1;
        }
        self.platform.add_share(&path, &name).await?;
        if !self
            .platform
            .list_shares()
            .await?
            .iter()
            .any(|s| s.id == name && s.path == path)
        {
            return Err(AppError::new(
                "VERIFY_FAILED",
                "无法确认目录已共享，请刷新列表检查",
            ));
        }
        Ok(())
    }
    pub async fn remove(&self, id: &str) -> Result<()> {
        let _lock = self
            .mutation
            .try_lock()
            .map_err(|_| AppError::new("BUSY", "操作正在进行中"))?;
        let start = std::time::Instant::now();
        let result = async {
            self.require_enabled().await?;
            if !self
                .platform
                .list_shares()
                .await?
                .iter()
                .any(|s| s.id == id)
            {
                return Err(AppError::new("NOT_FOUND", "该共享已不存在，请刷新列表"));
            }
            self.platform.remove_share(id).await?;
            if self
                .platform
                .list_shares()
                .await?
                .iter()
                .any(|s| s.id == id)
            {
                return Err(AppError::new(
                    "VERIFY_FAILED",
                    "无法确认共享已移除，请刷新检查",
                ));
            }
            Ok(())
        }
        .await;
        audit::record(
            "remove_share",
            Some(id),
            result.as_ref().err(),
            start.elapsed().as_millis(),
        );
        result
    }
    async fn require_enabled(&self) -> Result<()> {
        if self.platform.get_service_status().await? != ServiceStatus::Enabled {
            return Err(AppError::new("SERVICE_DISABLED", "请先开启共享服务"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    #[derive(Default)]
    struct Data {
        enabled: bool,
        shares: Vec<crate::model::ShareItem>,
        writes: Vec<String>,
        cancel: bool,
    }
    struct Fake(Arc<Mutex<Data>>);
    #[async_trait::async_trait]
    impl SmbPlatform for Fake {
        async fn get_service_status(&self) -> Result<ServiceStatus> {
            Ok(if self.0.lock().unwrap().enabled {
                ServiceStatus::Enabled
            } else {
                ServiceStatus::Disabled
            })
        }
        async fn enable_service(&self) -> Result<()> {
            self.0.lock().unwrap().enabled = true;
            Ok(())
        }
        async fn disable_service(&self) -> Result<()> {
            self.0.lock().unwrap().enabled = false;
            Ok(())
        }
        async fn list_shares(&self) -> Result<Vec<crate::model::ShareItem>> {
            Ok(self.0.lock().unwrap().shares.clone())
        }
        async fn add_share(&self, path: &str, name: &str) -> Result<()> {
            let mut d = self.0.lock().unwrap();
            if d.cancel {
                return Err(AppError::new("USER_CANCELED", "取消"));
            }
            d.writes.push(path.into());
            d.shares.push(crate::model::ShareItem {
                id: name.into(),
                name: name.into(),
                path: path.into(),
                read_only: false,
                guest_access: false,
                exists: true,
            });
            Ok(())
        }
        async fn remove_share(&self, name: &str) -> Result<()> {
            let mut d = self.0.lock().unwrap();
            d.writes.push(name.into());
            d.shares.retain(|s| s.id != name);
            Ok(())
        }
        async fn get_address(&self) -> Result<(String, bool)> {
            Ok(("smb://10.0.0.1".into(), false))
        }
    }
    fn setup() -> (Service, Arc<Mutex<Data>>) {
        let d = Arc::new(Mutex::new(Data {
            enabled: true,
            ..Default::default()
        }));
        (Service::new(Box::new(Fake(d.clone()))), d)
    }
    fn share(path: &str) -> crate::model::ShareItem {
        crate::model::ShareItem {
            id: "record".into(),
            name: "visible-name".into(),
            path: path.into(),
            read_only: false,
            guest_access: false,
            exists: false,
        }
    }
    #[tokio::test]
    async fn disabling_preserves_all_records() {
        let (s, d) = setup();
        d.lock().unwrap().shares.push(share("/missing"));
        s.set_enabled(false).await.unwrap();
        let d = d.lock().unwrap();
        assert!(!d.enabled);
        assert_eq!(d.shares.len(), 1);
        assert!(d.writes.is_empty());
    }
    #[tokio::test]
    async fn disabled_rejects_add_and_remove_before_writes() {
        let (s, d) = setup();
        d.lock().unwrap().enabled = false;
        assert_eq!(s.add("/tmp").await.unwrap_err().code, "SERVICE_DISABLED");
        assert_eq!(
            s.remove("record").await.unwrap_err().code,
            "SERVICE_DISABLED"
        );
        assert!(d.lock().unwrap().writes.is_empty());
    }
    #[tokio::test]
    async fn stale_directory_removed_by_record_name() {
        let (s, d) = setup();
        d.lock().unwrap().shares.push(share("/missing"));
        s.remove("record").await.unwrap();
        let d = d.lock().unwrap();
        assert_eq!(d.writes, vec!["record"]);
        assert!(d.shares.is_empty());
    }
    #[tokio::test]
    async fn duplicate_is_rejected_before_privileged_call() {
        let (s, d) = setup();
        let dir = tempfile::tempdir().unwrap();
        let path = security::canonical_directory(dir.path().to_str().unwrap()).unwrap();
        d.lock().unwrap().shares.push(share(&path));
        assert_eq!(s.add(&path).await.unwrap_err().code, "DUPLICATE");
        assert!(d.lock().unwrap().writes.is_empty());
    }
    #[tokio::test]
    async fn cancellation_keeps_existing_records() {
        let (s, d) = setup();
        d.lock().unwrap().cancel = true;
        let dir = tempfile::tempdir().unwrap();
        assert!(s
            .add(dir.path().to_str().unwrap())
            .await
            .unwrap_err()
            .canceled());
        assert!(d.lock().unwrap().shares.is_empty());
    }
    #[tokio::test]
    async fn busy_rejects_concurrent_write() {
        let (s, d) = setup();
        let _lock = s.mutation.lock().await;
        assert_eq!(s.set_enabled(false).await.unwrap_err().code, "BUSY");
        assert!(d.lock().unwrap().enabled);
    }
}
