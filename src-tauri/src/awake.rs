use crate::{
    error::{AppError, Result},
    model::ServiceStatus,
};
use std::{path::PathBuf, process::Child, sync::Mutex};

#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};

pub trait WakeDriver: Send {
    fn acquire(&mut self) -> Result<()>;
    fn release(&mut self);
}
#[derive(Default)]
pub struct NativeDriver {
    child: Option<Child>,
}
impl WakeDriver for NativeDriver {
    fn acquire(&mut self) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            // Only idle SYSTEM sleep is inhibited. No -d or -u: the display may sleep normally.
            // Bind lifetime to this app so force quit/crash also releases the assertion.
            let child = Command::new("/usr/bin/caffeinate")
                .args(["-i", "-w", &std::process::id().to_string()])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| AppError::new("WAKE_FAILED", format!("无法保持唤醒：{e}")))?;
            self.child = Some(child);
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(AppError::new(
                "PLATFORM_NOT_SUPPORTED",
                "当前平台暂不支持保持唤醒",
            ))
        }
    }
    fn release(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
impl Drop for NativeDriver {
    fn drop(&mut self) {
        self.release();
    }
}
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Preference {
    keep_awake: bool,
}
pub struct AwakeManager<D: WakeDriver> {
    path: PathBuf,
    enabled: bool,
    status: ServiceStatus,
    active: bool,
    driver: D,
}
impl<D: WakeDriver> AwakeManager<D> {
    pub fn new(path: PathBuf, driver: D) -> Self {
        Self {
            path,
            enabled: false,
            status: ServiceStatus::Unknown,
            active: false,
            driver,
        }
    }
    pub fn load(path: PathBuf, driver: D) -> Result<Self> {
        let preference = match std::fs::read(&path) {
            Ok(raw) => serde_json::from_slice::<Preference>(&raw).map_err(|_| {
                AppError::new(
                    "PREFERENCE_ERROR",
                    "无法读取保持唤醒设置，本次默认关闭；可在菜单中重新设置",
                )
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Preference::default(),
            Err(e) => return Err(AppError::io(e)),
        };
        let mut m = Self::new(path, driver);
        m.enabled = preference.keep_awake;
        Ok(m)
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn set_status(&mut self, status: ServiceStatus) -> Result<()> {
        self.status = status;
        self.reconcile()
    }
    fn reconcile(&mut self) -> Result<()> {
        let desired = self.enabled && self.status == ServiceStatus::Enabled;
        if desired && !self.active {
            self.driver.acquire()?;
            self.active = true;
        } else if !desired {
            self.stop();
        }
        Ok(())
    }
    pub fn toggle_preference(&mut self, status: ServiceStatus) -> Result<()> {
        self.status = status;
        self.set_preference(!self.enabled)
    }
    pub fn set_preference(&mut self, enabled: bool) -> Result<()> {
        let previous = self.enabled;
        self.enabled = enabled;
        if let Err(e) = self.reconcile() {
            self.enabled = previous;
            return Err(e);
        }
        let save = (|| -> Result<()> {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent).map_err(AppError::io)?;
            }
            let tmp = self.path.with_extension("json.tmp");
            let data = serde_json::to_vec(&Preference {
                keep_awake: enabled,
            })
            .map_err(AppError::io)?;
            std::fs::write(&tmp, data).map_err(AppError::io)?;
            std::fs::rename(&tmp, &self.path).map_err(AppError::io)?;
            Ok(())
        })();
        if let Err(e) = save {
            self.enabled = previous;
            let _ = self.reconcile();
            return Err(e);
        }
        Ok(())
    }
    pub fn stop(&mut self) {
        if self.active {
            self.driver.release();
            self.active = false;
        }
    }
}
impl<D: WakeDriver> Drop for AwakeManager<D> {
    fn drop(&mut self) {
        self.stop();
    }
}
pub struct AwakeState(pub Mutex<AwakeManager<NativeDriver>>);

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    #[derive(Default)]
    struct Fake {
        starts: Arc<AtomicUsize>,
        stops: Arc<AtomicUsize>,
        fail: bool,
    }
    impl WakeDriver for Fake {
        fn acquire(&mut self) -> Result<()> {
            if self.fail {
                return Err(AppError::new("WAKE_FAILED", "failed"));
            }
            self.starts.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn release(&mut self) {
            self.stops.fetch_add(1, Ordering::SeqCst);
        }
    }
    #[test]
    fn defaults_off_and_starts_only_for_enabled_service() {
        let d = tempfile::tempdir().unwrap();
        let driver = Fake::default();
        let starts = driver.starts.clone();
        let stops = driver.stops.clone();
        let mut manager = AwakeManager::load(d.path().join("awake.json"), driver).unwrap();
        assert!(!manager.enabled());
        manager.set_status(ServiceStatus::Enabled).unwrap();
        assert_eq!(starts.load(Ordering::SeqCst), 0);
        manager.set_preference(true).unwrap();
        manager.set_status(ServiceStatus::Enabled).unwrap();
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        manager.set_status(ServiceStatus::Disabled).unwrap();
        assert_eq!(stops.load(Ordering::SeqCst), 1);
        manager.set_status(ServiceStatus::Enabled).unwrap();
        assert_eq!(starts.load(Ordering::SeqCst), 2);
        manager.set_status(ServiceStatus::Unknown).unwrap();
        assert_eq!(stops.load(Ordering::SeqCst), 2);
    }
    #[test]
    fn preference_survives_restart_but_assertion_does_not() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("awake.json");
        let driver = Fake::default();
        let stops = driver.stops.clone();
        {
            let mut m = AwakeManager::load(path.clone(), driver).unwrap();
            m.set_status(ServiceStatus::Enabled).unwrap();
            m.set_preference(true).unwrap();
        }
        assert_eq!(stops.load(Ordering::SeqCst), 1);
        let driver = Fake::default();
        let starts = driver.starts.clone();
        let mut m = AwakeManager::load(path, driver).unwrap();
        assert!(m.enabled());
        assert_eq!(starts.load(Ordering::SeqCst), 0);
        m.set_status(ServiceStatus::Enabled).unwrap();
        m.set_preference(false).unwrap();
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        assert!(!m.active);
    }
    #[test]
    fn cannot_enable_when_driver_fails() {
        let d = tempfile::tempdir().unwrap();
        let mut m = AwakeManager::load(
            d.path().join("awake.json"),
            Fake {
                fail: true,
                ..Default::default()
            },
        )
        .unwrap();
        m.set_status(ServiceStatus::Enabled).unwrap();
        assert!(m.set_preference(true).is_err());
        assert!(!m.enabled());
        assert!(!m.active);
    }
    #[test]
    fn failed_save_rolls_back_runtime_state() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("awake.json");
        let mut m = AwakeManager::load(path.clone(), Fake::default()).unwrap();
        m.set_status(ServiceStatus::Enabled).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(m.set_preference(true).is_err());
        assert!(!m.enabled());
        assert!(!m.active);
    }
    #[test]
    fn shutdown_releases_without_forgetting_preference() {
        let d = tempfile::tempdir().unwrap();
        let mut m = AwakeManager::load(d.path().join("awake.json"), Fake::default()).unwrap();
        m.set_status(ServiceStatus::Enabled).unwrap();
        m.set_preference(true).unwrap();
        m.stop();
        assert!(!m.active);
        assert!(m.enabled());
    }
}

#[cfg(all(test, target_os = "macos"))]
mod native_tests {
    use super::*;
    #[test]
    fn native_child_is_alive_then_reaped_on_release() {
        let mut driver = NativeDriver::default();
        driver.acquire().unwrap();
        let child = driver.child.as_mut().unwrap();
        let pid = child.id();
        assert!(child.try_wait().unwrap().is_none());
        driver.release();
        assert!(!Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
    }
}
#[cfg(test)]
mod failure_toggle_tests {
    use super::*;
    #[test]
    fn persisted_enabled_can_be_unchecked_even_if_acquisition_is_broken() {
        struct Broken;
        impl WakeDriver for Broken {
            fn acquire(&mut self) -> Result<()> {
                Err(AppError::new("WAKE_FAILED", "broken"))
            }
            fn release(&mut self) {}
        }
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("awake.json");
        std::fs::write(&path, r#"{"keep_awake":true}"#).unwrap();
        let mut manager = AwakeManager::load(path.clone(), Broken).unwrap();
        manager.toggle_preference(ServiceStatus::Enabled).unwrap();
        assert!(!manager.enabled());
        assert!(!AwakeManager::load(path, Broken).unwrap().enabled());
    }
}
