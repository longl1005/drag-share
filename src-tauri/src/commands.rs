use crate::{
    audit, desktop,
    error::{AppError, Result},
    model::{Outcome, Snapshot},
    security,
    service::Service,
};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
pub struct Interaction<'a> {
    app: AppHandle,
    service: &'a Service,
}
impl<'a> Interaction<'a> {
    pub fn begin(app: &AppHandle, service: &'a Service) -> Result<Self> {
        service
            .native_dialog
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| AppError::new("BUSY", "操作正在进行中"))?;
        let _ = app.emit("native-busy", true);
        Ok(Self {
            app: app.clone(),
            service,
        })
    }
}
impl Drop for Interaction<'_> {
    fn drop(&mut self) {
        self.service.native_dialog.store(false, Ordering::SeqCst);
        let _ = self.app.emit("native-busy", false);
    }
}
fn outcome(result: Result<()>) -> Result<Outcome> {
    match result {
        Ok(()) => Ok(Outcome { canceled: false }),
        Err(e) if e.canceled() => Ok(Outcome { canceled: true }),
        Err(e) => Err(e),
    }
}
#[tauri::command]
pub async fn snapshot(app: AppHandle, state: State<'_, Service>) -> Result<Snapshot> {
    let s = state.snapshot().await;
    desktop::update(&app, s.status);
    Ok(s)
}
#[tauri::command]
pub async fn set_enabled(
    app: AppHandle,
    state: State<'_, Service>,
    enabled: bool,
) -> Result<Outcome> {
    let _interaction = Interaction::begin(&app, &state)?;
    let r = state.set_enabled(enabled).await;
    let s = state.snapshot().await;
    desktop::update(&app, s.status);
    let _ = app.emit("refresh", ());
    outcome(r)
}
#[tauri::command]
pub async fn choose_directory(app: AppHandle, state: State<'_, Service>) -> Result<Option<String>> {
    let _interaction = Interaction::begin(&app, &state)?;
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .set_title("选择要共享的文件夹")
            .blocking_pick_folder()
            .and_then(|p| p.into_path().ok())
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .map_err(AppError::io)
}
#[tauri::command]
pub async fn add_share(app: AppHandle, state: State<'_, Service>, path: String) -> Result<Outcome> {
    let _interaction = Interaction::begin(&app, &state)?;
    let r = state.add(&path).await;
    let _ = app.emit("refresh", ());
    outcome(r)
}
#[tauri::command]
pub async fn remove_share(
    app: AppHandle,
    state: State<'_, Service>,
    id: String,
) -> Result<Outcome> {
    let _interaction = Interaction::begin(&app, &state)?;
    let item = state
        .platform
        .list_shares()
        .await?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| AppError::new("NOT_FOUND", "该共享已不存在，请刷新列表"))?;
    let handle = app.clone();
    let name = item.name;
    let confirmed = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .message(format!(
                "确定要停止共享目录「{name}」吗？\n其他设备的连接会断开，本机文件不会删除。"
            ))
            .title("取消共享")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "停止共享".into(),
                "保留共享".into(),
            ))
            .blocking_show()
    })
    .await
    .map_err(AppError::io)?;
    if !confirmed {
        return Ok(Outcome { canceled: true });
    }
    let r = state.remove(&id).await;
    let _ = app.emit("refresh", ());
    outcome(r)
}
#[tauri::command]
pub async fn copy_address(
    app: AppHandle,
    state: State<'_, Service>,
    id: Option<String>,
) -> Result<()> {
    let start = std::time::Instant::now();
    let (address, _) = state.platform.get_address().await?;
    let address = if let Some(id) = id {
        let item = state
            .platform
            .list_shares()
            .await?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| AppError::new("NOT_FOUND", "共享已不存在，请刷新列表"))?;
        security::share_url(&address, &item.name)
    } else {
        address
    };
    let r = app.clipboard().write_text(address).map_err(AppError::io);
    audit::record(
        "copy_address",
        None,
        r.as_ref().err(),
        start.elapsed().as_millis(),
    );
    r
}
#[tauri::command]
pub async fn reveal_share(state: State<'_, Service>, id: String) -> Result<()> {
    let item = state
        .platform
        .list_shares()
        .await?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| AppError::new("NOT_FOUND", "共享已不存在"))?;
    if !std::path::Path::new(&item.path).is_dir() {
        return Err(AppError::new("NOT_FOUND", "原目录已不存在，可以取消此共享"));
    }
    #[cfg(target_os = "macos")]
    {
        let r = crate::process::read("/usr/bin/open", &["-R", &item.path]).await?;
        if !r.success {
            return Err(AppError::new("OPEN_FAILED", "无法在访达中显示目录"));
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(AppError::new(
            "PLATFORM_NOT_SUPPORTED",
            "当前平台正在适配中",
        ))
    }
}
#[tauri::command]
pub async fn open_settings() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let r = crate::process::read(
            "/usr/bin/open",
            &["x-apple.systempreferences:com.apple.Sharing-Settings.extension"],
        )
        .await?;
        if r.success {
            Ok(())
        } else {
            Err(AppError::new("OPEN_FAILED", "无法打开系统共享设置"))
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(AppError::new(
            "PLATFORM_NOT_SUPPORTED",
            "当前平台正在适配中",
        ))
    }
}
#[tauri::command]
pub fn hide_panel(app: AppHandle) -> Result<()> {
    if let Some(w) = app.get_webview_window("main") {
        w.hide().map_err(AppError::io)?;
    }
    Ok(())
}
#[tauri::command]
pub fn request_quit(app: AppHandle, state: State<'_, Service>) -> Result<()> {
    if state.busy() {
        return Err(AppError::new("BUSY", "请等待当前操作完成后退出"));
    }
    app.exit(0);
    Ok(())
}
