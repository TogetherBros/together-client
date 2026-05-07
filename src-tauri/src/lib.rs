use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{Emitter, Manager};

// ── Shared state ──────────────────────────────────────────────────────────────

struct OverlayState {
    character_zones: Vec<[i32; 4]>,
    user_ids: Vec<String>,
    is_dragging: bool,
    drag_user_idx: usize,
    last_drag_pos: (i32, i32),
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            character_zones: Vec::new(),
            user_ids: Vec::new(),
            is_dragging: false,
            drag_user_idx: 0,
            last_drag_pos: (0, 0),
        }
    }
}

type SharedState = Arc<Mutex<OverlayState>>;

// ── Window helpers ────────────────────────────────────────────────────────────

fn restore_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_always_on_top(false);
        let _ = w.set_ignore_cursor_events(false);
        let _ = w.set_size(tauri::Size::Logical(tauri::LogicalSize { width: 460.0, height: 660.0 }));
        let _ = w.center();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

// ── Cursor position (Windows) ─────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn get_cursor_pos() -> (i32, i32) {
    use winapi::shared::windef::POINT;
    use winapi::um::winuser::GetCursorPos;
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        (pt.x, pt.y)
    }
}

#[cfg(not(target_os = "windows"))]
fn get_cursor_pos() -> (i32, i32) { (0, 0) }

// ── Left mouse button state (Windows) ────────────────────────────────────────

#[cfg(target_os = "windows")]
fn is_lmb_down() -> bool {
    use winapi::um::winuser::GetAsyncKeyState;
    unsafe { (GetAsyncKeyState(0x01) as u16 & 0x8000) != 0 }
}

#[cfg(not(target_os = "windows"))]
fn is_lmb_down() -> bool { false }

// ── Drag monitor ──────────────────────────────────────────────────────────────
// 오버레이는 항상 passthrough (set_ignore_cursor_events 토글 없음 → 글리치 없음).
// Rust 가 OS 레벨에서 마우스 버튼+위치를 폴링해 드래그를 감지하고
// 물리 픽셀 델타를 drag-move 이벤트로 오버레이 JS 에 전달.

fn drag_monitor(app: tauri::AppHandle, state: SharedState) {
    enum Action {
        None,
        StartDrag(String),
        MoveDrag(String, i32, i32),
        EndDrag(String),
    }

    loop {
        thread::sleep(Duration::from_millis(16)); // ~60 fps

        let Some(overlay) = app.get_webview_window("overlay") else { continue };
        if !overlay.is_visible().unwrap_or(false) { continue; }

        let (cx, cy) = get_cursor_pos();
        let lmb = is_lmb_down();

        let action = {
            let mut s = state.lock().unwrap();

            if !s.is_dragging && lmb {
                let idx = s.character_zones.iter().position(|z| {
                    cx >= z[0] && cx < z[0] + z[2] && cy >= z[1] && cy < z[1] + z[3]
                });
                if let Some(i) = idx {
                    let uid = s.user_ids.get(i).cloned().unwrap_or_default();
                    s.is_dragging = true;
                    s.drag_user_idx = i;
                    s.last_drag_pos = (cx, cy);
                    Action::StartDrag(uid)
                } else {
                    Action::None
                }
            } else if s.is_dragging {
                if lmb {
                    let dx = cx - s.last_drag_pos.0;
                    let dy = cy - s.last_drag_pos.1;
                    if dx != 0 || dy != 0 {
                        let uid = s.user_ids.get(s.drag_user_idx).cloned().unwrap_or_default();
                        s.last_drag_pos = (cx, cy);
                        Action::MoveDrag(uid, dx, dy)
                    } else {
                        Action::None
                    }
                } else {
                    let uid = s.user_ids.get(s.drag_user_idx).cloned().unwrap_or_default();
                    s.is_dragging = false;
                    Action::EndDrag(uid)
                }
            } else {
                Action::None
            }
        };

        match action {
            Action::StartDrag(uid) => { let _ = overlay.emit("drag-start", uid); }
            Action::MoveDrag(uid, dx, dy) => { let _ = overlay.emit("drag-move", (uid, dx, dy)); }
            Action::EndDrag(uid) => { let _ = overlay.emit("drag-end", uid); }
            Action::None => {}
        }
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn enter_overlay(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") { let _ = w.hide(); }
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.set_ignore_cursor_events(true);
        let _ = w.show();
    }
    let mut s = state.lock().unwrap();
    s.character_zones.clear();
    s.user_ids.clear();
    s.is_dragging = false;
    Ok(())
}

#[tauri::command]
fn leave_overlay(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("overlay") { let _ = w.hide(); }
    restore_window(&app);
    let mut s = state.lock().unwrap();
    s.character_zones.clear();
    s.user_ids.clear();
    s.is_dragging = false;
    Ok(())
}

#[tauri::command]
fn update_character_zones(
    zones: Vec<Vec<i32>>,
    user_ids: Vec<String>,
    state: tauri::State<'_, SharedState>,
) {
    let mut s = state.lock().unwrap();
    s.character_zones = zones
        .into_iter()
        .filter_map(|z| if z.len() == 4 { Some([z[0], z[1], z[2], z[3]]) } else { None })
        .collect();
    s.user_ids = user_ids;
}

#[tauri::command]
fn get_active_app() -> String {
    get_active_app_internal()
}

// ── Active app detection (Windows) ───────────────────────────────────────────

#[cfg(target_os = "windows")]
fn get_active_app_internal() -> String {
    use winapi::shared::minwindef::DWORD;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::OpenProcess;
    use winapi::um::winbase::QueryFullProcessImageNameW;
    use winapi::um::winnt::PROCESS_QUERY_LIMITED_INFORMATION;
    use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() { return "접속 중".to_string(); }

        let mut title_buf = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 512);
        let title = if title_len > 0 {
            String::from_utf16_lossy(&title_buf[..title_len as usize])
        } else { String::new() };

        let mut pid: DWORD = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 { return "접속 중".to_string(); }

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() { return "접속 중".to_string(); }

        let mut exe_buf = [0u16; 512];
        let mut size: DWORD = 512;
        let ok = QueryFullProcessImageNameW(handle, 0, exe_buf.as_mut_ptr(), &mut size);
        CloseHandle(handle);

        if ok == 0 || size == 0 { return "접속 중".to_string(); }

        let path = String::from_utf16_lossy(&exe_buf[..size as usize]);
        let exe = path.split('\\').last().unwrap_or("")
            .to_lowercase()
            .trim_end_matches(".exe")
            .to_string();

        exe_to_app_name(&exe, &title)
    }
}

#[cfg(not(target_os = "windows"))]
fn get_active_app_internal() -> String { "접속 중".to_string() }

fn exe_to_app_name(exe: &str, title: &str) -> String {
    let is_browser = matches!(exe, "chrome"|"msedge"|"firefox"|"whale"|"opera"|"brave"|"vivaldi");
    if is_browser {
        let tl = title.to_lowercase();
        if tl.contains("youtube") { return "유튜브 시청 중".to_string(); }
        if tl.contains("netflix") { return "넷플릭스 시청 중".to_string(); }
        if tl.contains("twitch")  { return "트위치 시청 중".to_string(); }
        if tl.contains("github")  { return "GitHub 보는 중".to_string(); }
        if tl.contains("notion")  { return "Notion 작업 중".to_string(); }
        if tl.contains("figma")   { return "Figma 작업 중".to_string(); }
        let browser = match exe {
            "chrome"=>"Chrome","msedge"=>"Edge","firefox"=>"Firefox",
            "whale"=>"Whale","opera"=>"Opera","brave"=>"Brave",_=>"브라우저",
        };
        return format!("{} 실행 중", browser);
    }
    let name = match exe {
        "idea64"|"idea"=>"IntelliJ IDEA","code"=>"VS Code",
        "pycharm64"|"pycharm"=>"PyCharm","webstorm64"|"webstorm"=>"WebStorm",
        "clion64"|"clion"=>"CLion","devenv"=>"Visual Studio",
        "discord"=>"Discord","spotify"=>"Spotify","vlc"=>"VLC",
        "notepad"=>"메모장","notepad++"=>"Notepad++","explorer"=>"파일 탐색기",
        "slack"=>"Slack","zoom"=>"Zoom","obs64"|"obs32"|"obs"=>"OBS",
        "steam"=>"Steam","javaw"|"minecraft"|"minecraftlauncher"=>"마인크래프트",
        "windowsterminal"|"terminal"=>"터미널","powershell"=>"PowerShell",
        "cmd"=>"명령 프롬프트","kakaotalk"=>"카카오톡",
        _=>return format!("{} 실행 중", capitalize_first(exe)),
    };
    format!("{} 실행 중", name)
}

fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

// ── App setup ─────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shared_state: SharedState = Arc::new(Mutex::new(OverlayState::default()));

    tauri::Builder::default()
        .manage(shared_state.clone())
        .setup(move |app| {
            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::TrayIconBuilder;

            let app_handle = app.handle().clone();
            let state_for_thread = shared_state.clone();
            thread::spawn(move || drag_monitor(app_handle, state_for_thread));

            tauri::WebviewWindowBuilder::new(
                app,
                "overlay",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .maximized(true)
            .resizable(false)
            .shadow(false)
            .focused(false)
            .visible(false)
            .build()?;

            let app_handle2 = app.handle().clone();
            if let Some(window) = app.get_webview_window("main") {
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(w) = app_handle2.get_webview_window("main") {
                            let _ = w.hide();
                        }
                    }
                });
            }

            let show  = MenuItem::with_id(app, "show",  "열기",     true, None::<&str>)?;
            let leave = MenuItem::with_id(app, "leave", "방 나가기", true, None::<&str>)?;
            let quit  = MenuItem::with_id(app, "quit",  "종료",      true, None::<&str>)?;
            let menu  = Menu::with_items(app, &[&show, &leave, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("overlay") { let _ = w.hide(); }
                        restore_window(app);
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.emit("open-chat", ());
                        }
                    }
                    "leave" => {
                        if let Some(w) = app.get_webview_window("overlay") { let _ = w.hide(); }
                        restore_window(app);
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.emit("leave-overlay", ());
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            enter_overlay,
            leave_overlay,
            update_character_zones,
            get_active_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
