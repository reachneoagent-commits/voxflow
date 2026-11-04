use crate::managers::history::HistoryEntry;
use crate::meeting::{MeetingManager, MeetingStatus};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
#[specta::specta]
pub fn start_meeting(
    app: AppHandle,
    meeting: State<'_, Arc<MeetingManager>>,
) -> Result<MeetingStatus, String> {
    let status = meeting.start(&app);
    crate::tray::update_tray_menu(&app);
    status
}

#[tauri::command]
#[specta::specta]
pub async fn stop_meeting(
    app: AppHandle,
    meeting: State<'_, Arc<MeetingManager>>,
) -> Result<HistoryEntry, String> {
    let meeting = Arc::clone(&meeting);
    let result = tauri::async_runtime::spawn_blocking({
        let app = app.clone();
        move || meeting.stop(&app)
    })
    .await
    .map_err(|e| e.to_string())?;
    crate::tray::update_tray_menu(&app);
    result
}

#[tauri::command]
#[specta::specta]
pub fn meeting_status(meeting: State<'_, Arc<MeetingManager>>) -> MeetingStatus {
    meeting.status()
}
