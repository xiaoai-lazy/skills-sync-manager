use crate::models::{AppErrorDto, UpdateInfoDto};
use std::sync::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_updater::{Update, UpdaterExt};

pub struct PendingUpdate(pub Mutex<Option<Update>>);

#[tauri::command]
pub async fn check_app_update(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> Result<Option<UpdateInfoDto>, AppErrorDto> {
    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(_) => return Ok(None),
    };

    let update = match updater.check().await {
        Ok(Some(update)) => update,
        Ok(None) => return Ok(None),
        Err(_) => return Ok(None),
    };

    let notes = update
        .body
        .as_ref()
        .map(|body| body.trim())
        .filter(|body| !body.is_empty())
        .map(|body| body.to_string());

    let info = UpdateInfoDto {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        notes,
    };

    *pending.0.lock().expect("pending update lock") = Some(update);
    Ok(Some(info))
}

#[tauri::command]
pub async fn install_app_update(pending: State<'_, PendingUpdate>) -> Result<(), AppErrorDto> {
    let update = pending
        .0
        .lock()
        .expect("pending update lock")
        .take()
        .ok_or_else(|| AppErrorDto {
            code: "noPendingUpdate".to_string(),
            message: "没有待安装的更新，请先检查更新".to_string(),
        })?;

    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|err| AppErrorDto {
            code: "updateInstallFailed".to_string(),
            message: format!("更新安装失败：{}", err),
        })
}
