use crate::managers::history::HistoryEntry;
use crate::meeting::{MeetingManager, MeetingStatus};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
#[specta::specta]
pub fn start_meeting(meeting: State<'_, Arc<MeetingManager>>) -> Result<MeetingStatus, String> {
    meeting.start()
}

#[tauri::command]
#[specta::specta]
pub async fn stop_meeting(
    app: AppHandle,
    meeting: State<'_, Arc<MeetingManager>>,
) -> Result<HistoryEntry, String> {
    let meeting = Arc::clone(&meeting);
    tauri::async_runtime::spawn_blocking(move || meeting.stop(&app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
#[specta::specta]
pub fn meeting_status(meeting: State<'_, Arc<MeetingManager>>) -> MeetingStatus {
    meeting.status()
}
