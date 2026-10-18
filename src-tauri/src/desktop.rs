use crate::{
    awake::{AwakeManager, AwakeState, NativeDriver},
    commands,
    error::{AppError, Result},
    model::ServiceStatus,
    service::Service,
};
use std::sync::Mutex;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
// A tray click first blurs the panel; retain a short timestamp to avoid reopening it on that click.
#[derive(Default)]
pub struct WindowState {
    pub last_blur: Mutex<Option<std::time::Instant>>,
    pub warning: Mutex<Option<String>>,
}
fn icon(status: ServiceStatus) -> tauri::image::Image<'static> {
    let mut pixels = vec![0u8; 22 * 22 * 4];
    for y in 0..22 {
        for x in 0..22 {
            let outline = ((3..=18).contains(&x) && (7..=17).contains(&y))
                && (x == 3 || x == 18 || y == 7 || y == 17);
            let arrow = (x == 10 && (3..=12).contains(&y))
                || ((7..=10).contains(&x) && y == x + 2)
                || ((10..=13).contains(&x) && y == 22 - x);
            if outline || arrow {
                let p = (y * 22 + x) * 4;
                pixels[p..p + 4].copy_from_slice(&[
                    30,
                    30,
                    32,
                    if status == ServiceStatus::Disabled {
                        125
                    } else {
                        255
                    },
                ]);
            }
            if status == ServiceStatus::Enabled
                && (x as i32 - 17).pow(2) + (y as i32 - 17).pow(2) <= 8
            {
                let p = (y * 22 + x) * 4;
                pixels[p..p + 4].copy_from_slice(&[46, 125, 50, 255]);
            }
        }
    }
    tauri::image::Image::new_owned(pixels, 22, 22)
}
fn menu(app: &AppHandle, status: ServiceStatus) -> tauri::Result<Menu<tauri::Wry>> {
    let toggle = MenuItem::with_id(
        app,
        "toggle",
        if status == ServiceStatus::Enabled {
            "关闭共享服务"
        } else {
            "开启共享服务"
        },
        status != ServiceStatus::Unknown,
        None::<&str>,
    )?;
    let add = MenuItem::with_id(
        app,
        "add",
        "添加共享目录…",
        status == ServiceStatus::Enabled,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, "settings", "系统共享设置…", true, None::<&str>)?;
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "开机自动启动",
        true,
        app.autolaunch().is_enabled().unwrap_or(false),
        None::<&str>,
    )?;
    let keep_awake = CheckMenuItem::with_id(
        app,
        "keep-awake",
        "共享时保持唤醒",
        cfg!(target_os = "macos"),
        app.state::<AwakeState>().0.lock().unwrap().enabled(),
        None::<&str>,
    )?;
    let about = MenuItem::with_id(app, "about", "关于 SMB 共享助手", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出应用", true, Some("CmdOrCtrl+Q"))?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    Menu::with_items(
        app,
        &[
            &toggle,
            &add,
            &sep1,
            &settings,
            &keep_awake,
            &autostart,
            &sep2,
            &about,
            &quit,
        ],
    )
}
pub fn update(app: &AppHandle, status: ServiceStatus, observation: u64) {
    app.state::<Service>().apply_observation(observation, || {
        if let Some(state) = app.try_state::<AwakeState>() {
            if let Err(e) = state.0.lock().unwrap().set_status(status) {
                let _ = app.emit("operation-error", e);
            }
        }
        if let Some(tray) = app.tray_by_id("main") {
            let _ = tray.set_icon(Some(icon(status)));
            let _ = tray.set_icon_as_template(status != ServiceStatus::Enabled);
            let _ = tray.set_tooltip(Some(match status {
                ServiceStatus::Enabled => "SMB 共享助手 · 共享已开启",
                ServiceStatus::Disabled => "SMB 共享助手 · 共享已关闭",
                ServiceStatus::Unknown => "SMB 共享助手 · 状态待确认",
            }));
            if let Ok(menu) = menu(app, status) {
                let _ = tray.set_menu(Some(menu));
            }
        }
    });
}
pub fn toggle_panel(app: &AppHandle) {
    if app.state::<Service>().busy() {
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
            return;
        }
        if app
            .state::<WindowState>()
            .last_blur
            .lock()
            .unwrap()
            .is_some_and(|t| t.elapsed().as_millis() < 180)
        {
            return;
        }
        show_panel(app);
    }
}
pub fn show_panel(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let tray = app.tray_by_id("main").and_then(|t| t.rect().ok().flatten());
    let scale = window.scale_factor().unwrap_or(1.0);
    let position = tray.as_ref().map(|r| r.position.to_physical::<f64>(scale));
    let monitor = position
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let factor = m.scale_factor();
        let work = m.work_area();
        let margin = 8.0 * factor;
        let w = 320.0 * factor;
        let h = 440.0 * factor;
        let left = work.position.x as f64;
        let top = work.position.y as f64;
        let right = left + work.size.width as f64;
        let bottom = top + work.size.height as f64;
        let (x, y) = if let Some(r) = tray {
            let p = r.position.to_physical::<f64>(factor);
            let s = r.size.to_physical::<f64>(factor);
            (
                p.x + s.width / 2.0 - w / 2.0,
                if p.y < top + work.size.height as f64 / 2.0 {
                    p.y + s.height + 6.0 * factor
                } else {
                    p.y - h - 6.0 * factor
                },
            )
        } else {
            (right - w - margin, top + margin)
        };
        let _ = window.set_position(PhysicalPosition::new(
            x.clamp(left + margin, (right - w - margin).max(left + margin)) as i32,
            y.clamp(top + margin, (bottom - h - margin).max(top + margin)) as i32,
        ));
    }
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit("panel-open", ());
}
fn dispatch(app: &AppHandle, id: &str) {
    let app = app.clone();
    let id = id.to_string();
    tauri::async_runtime::spawn(async move {
        let result: Result<()> = match id.as_str() {
            "toggle" => {
                let state = app.state::<Service>();
                match state.platform.get_service_status().await {
                    Ok(s) if s != ServiceStatus::Unknown => {
                        commands::set_enabled(app.clone(), state, s != ServiceStatus::Enabled)
                            .await
                            .map(|_| ())
                    }
                    Ok(_) => Err(AppError::new("UNKNOWN", "无法确认共享状态，请刷新后重试")),
                    Err(e) => Err(e),
                }
            }
            "add" => {
                show_panel(&app);
                match commands::choose_directory(app.clone(), app.state::<Service>()).await {
                    Ok(Some(path)) => {
                        commands::add_share(app.clone(), app.state::<Service>(), path)
                            .await
                            .map(|_| ())
                    }
                    Ok(None) => Ok(()),
                    Err(e) => Err(e),
                }
            }
            "settings" => commands::open_settings().await,
            "keep-awake" => {
                let service = app.state::<Service>();
                let observation = service.begin_observation();
                let status = service
                    .platform
                    .get_service_status()
                    .await
                    .unwrap_or(ServiceStatus::Unknown);
                let result = service
                    .apply_observation(observation, || {
                        app.state::<AwakeState>()
                            .0
                            .lock()
                            .unwrap()
                            .toggle_preference(status)
                    })
                    .unwrap_or_else(|| Err(AppError::new("BUSY", "共享状态已更新，请重试")));
                update(&app, status, observation);
                result
            }
            "autostart" => {
                let result = (|| {
                    let current = app.autolaunch().is_enabled()?;
                    if current {
                        app.autolaunch().disable()
                    } else {
                        app.autolaunch().enable()
                    }
                })()
                .map_err(AppError::io);
                let snapshot = app.state::<Service>().snapshot().await;
                update(&app, snapshot.status, snapshot.observation_id);
                result
            }
            "about" => {
                let state = app.state::<Service>();
                let result = if let Ok(_interaction) = commands::Interaction::begin(&app, &state) {
                    let handle = app.clone();
                    let _=tauri::async_runtime::spawn_blocking(move||handle.dialog().message("SMB 共享助手 0.1.2\n轻松管理本机 SMB 文件共享\n\n快捷键：Control + Option + S\n所有数据与日志仅保存在本机。\n关闭服务会保留共享目录配置。").title("关于 SMB 共享助手").blocking_show()).await;
                    Ok(())
                } else {
                    Err(AppError::new("BUSY", "操作正在进行中"))
                };
                result
            }
            "quit" => {
                if app.state::<Service>().busy() {
                    Err(AppError::new("BUSY", "请等待当前操作完成后退出"))
                } else {
                    app.exit(0);
                    Ok(())
                }
            }
            _ => Ok(()),
        };
        if let Err(e) = result {
            show_panel(&app);
            let _ = app.emit("operation-error", e);
        }
    });
}
pub fn setup(app: &mut tauri::App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    let handle = app.handle();
    handle.manage(WindowState::default());
    let path = handle.path().app_config_dir()?.join("awake.json");
    let manager = match AwakeManager::load(path.clone(), NativeDriver::default()) {
        Ok(manager) => manager,
        Err(e) => {
            *handle.state::<WindowState>().warning.lock().unwrap() = Some(e.message);
            AwakeManager::new(path, NativeDriver::default())
        }
    };
    handle.manage(AwakeState(Mutex::new(manager)));
    TrayIconBuilder::with_id("main")
        .icon(icon(ServiceStatus::Unknown))
        .icon_as_template(true)
        .tooltip("SMB 共享助手")
        .menu(&menu(handle, ServiceStatus::Unknown)?)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                toggle_panel(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| dispatch(app, event.id.as_ref()))
        .build(app)?;
    if let Some(window) = handle.get_webview_window("main") {
        #[cfg(target_os = "macos")]
        if let Err(e) = window_vibrancy::apply_vibrancy(
            &window,
            window_vibrancy::NSVisualEffectMaterial::Popover,
            None,
            Some(12.0),
        ) {
            eprintln!("Native vibrancy unavailable: {e}");
        }
        let app = handle.clone();
        window.on_window_event(move |event| match event {
            tauri::WindowEvent::Focused(false) => {
                if !app.state::<Service>().busy() {
                    *app.state::<WindowState>().last_blur.lock().unwrap() =
                        Some(std::time::Instant::now());
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.hide();
                    }
                }
            }
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = commands::hide_panel(app.clone());
            }
            _ => {}
        });
    }
    if let Err(e) = handle.global_shortcut().register("Control+Alt+KeyS") {
        *handle.state::<WindowState>().warning.lock().unwrap() =
            Some(format!("快捷键注册失败：{e}，仍可点击菜单栏图标使用"));
    }
    let app = handle.clone();
    tauri::async_runtime::spawn(async move {
        let s = app.state::<Service>().snapshot().await;
        update(&app, s.status, s.observation_id);
    });
    Ok(())
}
pub fn shortcut_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _, event| {
            if event.state == ShortcutState::Pressed {
                toggle_panel(app);
            }
        })
        .build()
}
#[tauri::command]
pub fn startup_warning(state: State<'_, WindowState>) -> Option<String> {
    state.warning.lock().unwrap().take()
}
