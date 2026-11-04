use crate::managers::history::{HistoryEntry, HistoryManager};
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

#[tauri::command]
#[specta::specta]
pub fn get_meetings(
    history_manager: State<'_, Arc<HistoryManager>>,
) -> Result<Vec<HistoryEntry>, String> {
    history_manager.get_meetings().map_err(|e| e.to_string())
}

/// Writes a meeting's transcript to a file the user picked in a save dialog.
#[tauri::command]
#[specta::specta]
pub fn save_meeting_file(path: String, contents: String) -> Result<(), String> {
    std::fs::write(&path, contents).map_err(|e| format!("Could not save the file: {e}"))
}
