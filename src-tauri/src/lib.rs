use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

const TRAY_ID: &str = "main";

// ── Shared state ──────────────────────────────────────────────────────────────

struct OverlayState {
    drag_enabled: bool,
    character_size: u8,
    hidden_users: HashSet<String>,
    tray_user_ids: Vec<String>,
    tray_user_labels: Vec<String>,
    char_user_ids: Vec<String>,
    users_json: String,
    notifications_muted: bool,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            drag_enabled: true,
            character_size: 2,
            hidden_users: HashSet::new(),
            tray_user_ids: Vec::new(),
            tray_user_labels: Vec::new(),
            char_user_ids: Vec::new(),
            users_json: "[]".to_string(),
            notifications_muted: false,
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

fn emit_to_char_windows<S: serde::Serialize + Clone>(
    app: &tauri::AppHandle,
    state: &SharedState,
    event: &str,
    payload: S,
) {
    let ids = state.lock().unwrap().char_user_ids.clone();
    for uid in &ids {
        if let Some(w) = app.get_webview_window(&format!("char_{}", uid)) {
            let _ = w.emit(event, payload.clone());
        }
    }
}

fn destroy_char_windows(app: &tauri::AppHandle, state: &SharedState) {
    let ids = state.lock().unwrap().char_user_ids.clone();
    for uid in &ids {
        if let Some(w) = app.get_webview_window(&format!("char_{}", uid)) {
            let _ = w.destroy();
        }
    }
    state.lock().unwrap().char_user_ids.clear();
}

/// 스케일 레벨별 창 크기 (시각적 콘텐츠 기준으로 최소화)
/// transform-origin: bottom center 이므로 시각 높이 = DOM높이 × scale
fn char_window_size(level: u8) -> (f64, f64) {
    match level {
        1 => (140.0, 130.0),  // scale 0.7
        3 => (245.0, 230.0),  // scale 1.35
        _ => (185.0, 180.0),  // scale 1.0 (기본)
    }
}

fn default_char_positions(app: &tauri::AppHandle, total: usize, size_level: u8) -> Vec<(f64, f64)> {
    let (screen_w, screen_h) = app.get_webview_window("main")
        .and_then(|w| w.primary_monitor().ok().flatten())
        .map(|m| {
            let size = m.size();
            let sf = m.scale_factor();
            (size.width as f64 / sf, size.height as f64 / sf)
        })
        .unwrap_or((1920.0, 1080.0));

    let (window_w, window_h) = char_window_size(size_level);
    let char_w  = 90.0_f64;
    let taskbar = 48.0_f64;

    (0..total).map(|i| {
        let spacing = if total > 1 {
            (screen_w - char_w * total as f64) / (total + 1) as f64
        } else {
            (screen_w - char_w) / 2.0
        };
        let cx = spacing + i as f64 * (char_w + spacing) + char_w / 2.0;
        let x  = (cx - window_w / 2.0).max(0.0);
        let y  = (screen_h - taskbar - window_h).max(0.0);
        (x, y)
    }).collect()
}

// ── Tray rebuild ──────────────────────────────────────────────────────────────

fn rebuild_tray(app: &tauri::AppHandle, state: &SharedState) {
    use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};

    let (drag_enabled, char_size, hidden_users, user_ids, user_labels, muted) = {
        let s = state.lock().unwrap();
        (s.drag_enabled, s.character_size, s.hidden_users.clone(),
         s.tray_user_ids.clone(), s.tray_user_labels.clone(), s.notifications_muted)
    };

    let Ok(show)  = MenuItem::with_id(app, "show",  "채팅창 키기",  true, None::<&str>) else { return };
    let Ok(sep0)  = PredefinedMenuItem::separator(app) else { return };
    let Ok(mute)  = CheckMenuItem::with_id(app, "mute-notify", "전체 알림 끄기", true, muted, None::<&str>) else { return };
    let Ok(sep_m) = PredefinedMenuItem::separator(app) else { return };
    let Ok(drag)  = CheckMenuItem::with_id(app, "toggle-drag", "드래그", true, drag_enabled, None::<&str>) else { return };
    let Ok(s_sm)  = CheckMenuItem::with_id(app, "size-1", "작게", true, char_size == 1, None::<&str>) else { return };
    let Ok(s_md)  = CheckMenuItem::with_id(app, "size-2", "보통", true, char_size == 2, None::<&str>) else { return };
    let Ok(s_lg)  = CheckMenuItem::with_id(app, "size-3", "크게", true, char_size == 3, None::<&str>) else { return };
    let Ok(s_sub) = Submenu::with_items(app, "캐릭터 크기", true, &[&s_sm, &s_md, &s_lg]) else { return };
    let Ok(sep1)  = PredefinedMenuItem::separator(app) else { return };
    let Ok(reset) = MenuItem::with_id(app, "reset", "캐릭터 위치 초기화", true, None::<&str>) else { return };
    let Ok(leave) = MenuItem::with_id(app, "leave", "방 나가기",          true, None::<&str>) else { return };
    let Ok(sep2)  = PredefinedMenuItem::separator(app) else { return };
    let Ok(quit)  = MenuItem::with_id(app, "quit",  "종료하기",           true, None::<&str>) else { return };

    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };

    if !user_ids.is_empty() {
        let checks: Vec<CheckMenuItem<tauri::Wry>> = user_ids.iter().zip(user_labels.iter())
            .enumerate()
            .map(|(i, (uid, label))| {
                let visible = !hidden_users.contains(uid.as_str());
                CheckMenuItem::with_id(app, format!("vis-{}", i), label, true, visible, None::<&str>).unwrap()
            })
            .collect();
        let refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
            checks.iter().map(|c| c as &dyn tauri::menu::IsMenuItem<tauri::Wry>).collect();
        let Ok(v_sub) = Submenu::with_items(app, "캐릭터 보기/숨기기", true, refs.as_slice()) else { return };
        if let Ok(menu) = Menu::with_items(app, &[&show, &sep0, &mute, &sep_m, &drag, &s_sub, &v_sub, &sep1, &reset, &leave, &sep2, &quit]) {
            let _ = tray.set_menu(Some(menu));
        }
    } else if let Ok(menu) = Menu::with_items(app, &[&show, &sep0, &mute, &sep_m, &drag, &s_sub, &sep1, &reset, &leave, &sep2, &quit]) {
        let _ = tray.set_menu(Some(menu));
    }
}

// ── Active app detection ──────────────────────────────────────────────────────

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
        let exe = path.split('\\').last().unwrap_or("").to_lowercase().trim_end_matches(".exe").to_string();
        exe_to_app_name(&exe, &title)
    }
}

#[cfg(target_os = "macos")]
fn get_active_app_internal() -> String {
    use objc2_app_kit::NSWorkspace;
    let ws = NSWorkspace::sharedWorkspace();
    let name = ws.frontmostApplication()
        .and_then(|a| a.localizedName())
        .map(|s| s.to_string());
    match name.as_deref() {
        Some(n) if !n.is_empty() => macos_app_to_display(n),
        _ => "앱 사용 중".to_string(),
    }
}

#[cfg(target_os = "macos")]
fn macos_app_to_display(name: &str) -> String {
    let lower = name.to_lowercase();
    let label = match lower.as_str() {
        "safari"|"chrome"|"firefox"|"arc"|"whale"|"opera"|"brave" => return format!("{} 실행 중", name),
        "xcode" => "Xcode 작업 중",
        "visual studio code"|"code" => "VS Code 작업 중",
        "intellij idea" => "IntelliJ IDEA 작업 중",
        "pycharm" => "PyCharm 작업 중",
        "webstorm" => "WebStorm 작업 중",
        "discord" => "Discord 중",
        "spotify" => "Spotify 듣는 중",
        "slack" => "Slack 중",
        "zoom" => "Zoom 중",
        "obs" => "OBS 중",
        "steam" => "Steam 중",
        "finder" => "파인더 사용 중",
        "terminal"|"iterm2"|"warp" => "터미널 중",
        "notion" => "Notion 작업 중",
        "figma" => "Figma 작업 중",
        _ => return format!("{} 실행 중", name),
    };
    label.to_string()
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn get_active_app_internal() -> String { "접속 중".to_string() }

#[cfg(target_os = "windows")]
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
        let b = match exe { "chrome"=>"Chrome","msedge"=>"Edge","firefox"=>"Firefox","whale"=>"Whale","opera"=>"Opera","brave"=>"Brave",_=>"브라우저" };
        return format!("{} 실행 중", b);
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

#[cfg(target_os = "windows")]
fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() { None => String::new(), Some(f) => f.to_uppercase().collect::<String>() + c.as_str() }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn show_main_window(app: tauri::AppHandle) { restore_window(&app); }

#[tauri::command]
fn quit_app(app: tauri::AppHandle) { app.exit(0); }

#[tauri::command]
fn enter_overlay(app: tauri::AppHandle) -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    { let _ = app; return Ok(false); }

    #[cfg(not(target_os = "linux"))]
    {
        if let Some(w) = app.get_webview_window("main") { let _ = w.hide(); }
        Ok(true)
    }
}

#[tauri::command]
fn leave_overlay(app: tauri::AppHandle, state: tauri::State<'_, SharedState>) -> Result<(), String> {
    destroy_char_windows(&app, &state);
    {
        let mut s = state.lock().unwrap();
        s.tray_user_ids.clear();
        s.tray_user_labels.clear();
        s.hidden_users.clear();
    }
    restore_window(&app);
    rebuild_tray(&app, &state);
    Ok(())
}

fn percent_encode_ascii(s: &str) -> String {
    s.chars().map(|c| match c {
        'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
        _ => format!("%{:02X}", c as u8),
    }).collect()
}

// 비ASCII 문자를 \uXXXX로 변환 → initialization_script에서 한국어 인코딩 문제 방지
fn unicode_escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        if ch.is_ascii() {
            out.push(ch);
        } else {
            let code = ch as u32;
            if code <= 0xFFFF {
                out.push_str(&format!("\\u{:04X}", code));
            } else {
                let code = code - 0x10000;
                let high = 0xD800 + (code >> 10);
                let low = 0xDC00 + (code & 0x3FF);
                out.push_str(&format!("\\u{:04X}\\u{:04X}", high, low));
            }
        }
    }
    out
}

#[tauri::command]
async fn sync_char_windows(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    user_ids: Vec<String>,
    positions: Vec<[f64; 2]>,
    users_json: String,  // JS에서 직접 전달 — Rust 공유 상태 타이밍 문제 완전 제거
) -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    { return Ok(false); }

    #[cfg(not(target_os = "linux"))]
    {
        // 전달받은 JSON을 공유 상태에도 저장 (get_user_for_window 폴백용)
        state.lock().unwrap().users_json = users_json.clone();

        let (size_level, existing_ids) = {
            let s = state.lock().unwrap();
            (s.character_size, s.char_user_ids.clone())
        };

        // Destroy windows for users who left
        for uid in &existing_ids {
            if !user_ids.contains(uid) {
                if let Some(w) = app.get_webview_window(&format!("char_{}", uid)) {
                    let _ = w.destroy();
                }
            }
        }

        // Create windows for new users — users_json comes directly from JS parameter
        let users_val: Vec<serde_json::Value> =
            serde_json::from_str(&users_json).unwrap_or_default();
        let drag_enabled = state.lock().unwrap().drag_enabled;
        let (win_w, win_h) = char_window_size(size_level);

        eprintln!("[Together] sync_char_windows: user_ids={:?} users_json_len={} parsed_count={}", user_ids, users_json.len(), users_val.len());

        for (i, uid) in user_ids.iter().enumerate() {
            if existing_ids.contains(uid) { continue; }
            let label = format!("char_{}", uid);
            if app.get_webview_window(&label).is_some() { continue; }

            let pos = positions.get(i).copied().unwrap_or([100.0 + i as f64 * 240.0, 700.0]);
            let is_hidden = state.lock().unwrap().hidden_users.contains(uid.as_str());

            // Find user data — passed as ?u= query param (percent-encoded JSON)
            let user_data = users_val.iter()
                .find(|u| u.get("userId").and_then(|v| v.as_str()) == Some(uid.as_str()))
                .map(|u| u.to_string())
                .unwrap_or_else(|| "null".to_string());

            eprintln!("[Together] char 창 생성: label={} user={}", label, &user_data[..user_data.len().min(80)]);

            let escaped = unicode_escape_json(&user_data);
            let init_script = format!("window.__TAURI_CHAR_USER__={};", escaped);
            let char_url = format!("char.html?u={}", percent_encode_ascii(&escaped));

            match tauri::WebviewWindowBuilder::new(
                &app,
                &label,
                tauri::WebviewUrl::App(char_url.into()),
            )
            .initialization_script(&init_script)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .focused(false)
            .visible(!is_hidden)
            .position(pos[0], pos[1])
            .inner_size(win_w, win_h)
            .build() {
                Ok(w) => {
                    // OS 레벨 클릭 투과: 드래그 비활성 시 창이 클릭을 가로채지 않음
                    let _ = w.set_ignore_cursor_events(!drag_enabled);
                    #[cfg(debug_assertions)]
                    let _ = w.open_devtools();
                }
                Err(e) => {
                    eprintln!("[Together] char 창 생성 실패 '{}': {}", label, e);
                }
            }
        }

        state.lock().unwrap().char_user_ids = user_ids;

        Ok(true)
    }
}

#[tauri::command]
fn update_overlay_users(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    user_ids: Vec<String>,
    user_labels: Vec<String>,
) {
    let changed = {
        let mut s = state.lock().unwrap();
        let changed = s.tray_user_ids != user_ids || s.tray_user_labels != user_labels;
        if changed {
            s.hidden_users.retain(|id| user_ids.contains(id));
            s.tray_user_ids = user_ids;
            s.tray_user_labels = user_labels;
        }
        changed
    };
    if changed {
        rebuild_tray(&app, &state);
    }
}

#[tauri::command]
fn store_users_json(state: tauri::State<'_, SharedState>, json: String) {
    eprintln!("[Together] store_users_json: {} chars", json.len());
    state.lock().unwrap().users_json = json;
}

#[tauri::command]
fn get_users_json(state: tauri::State<'_, SharedState>) -> String {
    state.lock().unwrap().users_json.clone()
}

#[tauri::command]
fn get_user_for_window(label: String, state: tauri::State<'_, SharedState>) -> String {
    let user_id = label.trim_start_matches("char_").to_string();
    let json = state.lock().unwrap().users_json.clone();
    let preview: String = json.chars().take(200).collect();
    eprintln!("[Together] get_user_for_window label={} userId={} json={}", label, user_id, preview);
    let users: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap_or_default();
    for user in users {
        if user.get("userId").and_then(|v| v.as_str()) == Some(user_id.as_str()) {
            return user.to_string();
        }
    }
    "null".to_string()
}

#[tauri::command]
fn get_active_app() -> String { get_active_app_internal() }

// ── App setup ─────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shared_state: SharedState = Arc::new(Mutex::new(OverlayState::default()));

    tauri::Builder::default()
        .manage(shared_state.clone())
        .setup(move |app| {
            use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
            use tauri::tray::TrayIconBuilder;

            #[cfg(debug_assertions)]
            if let Some(w) = app.get_webview_window("main") { w.open_devtools(); }

            let app_handle2 = app.handle().clone();
            if let Some(window) = app.get_webview_window("main") {
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(w) = app_handle2.get_webview_window("main") { let _ = w.hide(); }
                    }
                });
            }

            let show  = MenuItem::with_id(app, "show",  "채팅창 키기",       true, None::<&str>)?;
            let sep0  = PredefinedMenuItem::separator(app)?;
            let mute  = CheckMenuItem::with_id(app, "mute-notify", "전체 알림 끄기", true, false, None::<&str>)?;
            let sep_m = PredefinedMenuItem::separator(app)?;
            let drag  = CheckMenuItem::with_id(app, "toggle-drag", "드래그",  true, true,  None::<&str>)?;
            let s_sm  = CheckMenuItem::with_id(app, "size-1", "작게",         true, false, None::<&str>)?;
            let s_md  = CheckMenuItem::with_id(app, "size-2", "보통",         true, true,  None::<&str>)?;
            let s_lg  = CheckMenuItem::with_id(app, "size-3", "크게",         true, false, None::<&str>)?;
            let s_sub = Submenu::with_items(app, "캐릭터 크기", true, &[&s_sm, &s_md, &s_lg])?;
            let sep1  = PredefinedMenuItem::separator(app)?;
            let reset = MenuItem::with_id(app, "reset", "캐릭터 위치 초기화", true, None::<&str>)?;
            let leave = MenuItem::with_id(app, "leave", "방 나가기",          true, None::<&str>)?;
            let sep2  = PredefinedMenuItem::separator(app)?;
            let quit  = MenuItem::with_id(app, "quit",  "종료하기",           true, None::<&str>)?;
            let menu  = Menu::with_items(app, &[&show, &sep0, &mute, &sep_m, &drag, &s_sub, &sep1, &reset, &leave, &sep2, &quit])?;

            let state_for_tray = shared_state.clone();
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Together")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                                let _ = w.emit("open-chat", ());
                            }
                        }
                        "toggle-drag" => {
                            let enabled = {
                                let mut s = state_for_tray.lock().unwrap();
                                s.drag_enabled ^= true;
                                s.drag_enabled
                            };
                            // OS 레벨 클릭 투과 토글
                            let ids = state_for_tray.lock().unwrap().char_user_ids.clone();
                            for uid in &ids {
                                if let Some(w) = app.get_webview_window(&format!("char_{}", uid)) {
                                    let _ = w.set_ignore_cursor_events(!enabled);
                                }
                            }
                            emit_to_char_windows(app, &state_for_tray, "drag-enabled", enabled);
                            rebuild_tray(app, &state_for_tray);
                        }
                        id @ ("size-1" | "size-2" | "size-3") => {
                            let new_size: u8 = id.trim_start_matches("size-").parse().unwrap_or(2);
                            let (old_size, ids, hidden_users) = {
                                let mut s = state_for_tray.lock().unwrap();
                                let old = s.character_size;
                                s.character_size = new_size;
                                (old, s.char_user_ids.clone(), s.hidden_users.clone())
                            };
                            let (old_w, old_h) = char_window_size(old_size);
                            let (new_w, new_h) = char_window_size(new_size);
                            for uid in &ids {
                                if let Some(w) = app.get_webview_window(&format!("char_{}", uid)) {
                                    let was_visible = !hidden_users.contains(uid.as_str());
                                    if was_visible { let _ = w.hide(); }
                                    if let Ok(pos) = w.outer_position() {
                                        let sf = w.scale_factor().unwrap_or(1.0);
                                        let lx = pos.x as f64 / sf;
                                        let ly = pos.y as f64 / sf;
                                        let new_lx = lx + (old_w - new_w) / 2.0;
                                        let new_ly = ly + (old_h - new_h);
                                        let _ = w.set_position(tauri::Position::Logical(
                                            tauri::LogicalPosition { x: new_lx.max(0.0), y: new_ly.max(0.0) },
                                        ));
                                    }
                                    let _ = w.set_size(tauri::Size::Logical(
                                        tauri::LogicalSize { width: new_w, height: new_h },
                                    ));
                                    if was_visible { let _ = w.show(); }
                                }
                            }
                            emit_to_char_windows(app, &state_for_tray, "character-size-changed", new_size);
                            rebuild_tray(app, &state_for_tray);
                        }
                        "reset" => {
                            let (ids, size_level) = {
                                let s = state_for_tray.lock().unwrap();
                                (s.char_user_ids.clone(), s.character_size)
                            };
                            let positions = default_char_positions(app, ids.len(), size_level);
                            for (i, uid) in ids.iter().enumerate() {
                                if let Some(w) = app.get_webview_window(&format!("char_{}", uid)) {
                                    let (px, py) = positions[i];
                                    let _ = w.set_position(tauri::Position::Logical(
                                        tauri::LogicalPosition { x: px, y: py },
                                    ));
                                }
                            }
                        }
                        "leave" => {
                            destroy_char_windows(app, &state_for_tray);
                            {
                                let mut s = state_for_tray.lock().unwrap();
                                s.tray_user_ids.clear();
                                s.tray_user_labels.clear();
                                s.hidden_users.clear();
                            }
                            rebuild_tray(app, &state_for_tray);
                            restore_window(app);
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.emit("leave-overlay", ());
                            }
                        }
                        "mute-notify" => {
                            let muted = {
                                let mut s = state_for_tray.lock().unwrap();
                                s.notifications_muted ^= true;
                                s.notifications_muted
                            };
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.emit("mute-changed", muted);
                            }
                            rebuild_tray(app, &state_for_tray);
                        }
                        "quit" => app.exit(0),
                        id if id.starts_with("vis-") => {
                            let idx: usize = id.trim_start_matches("vis-").parse().unwrap_or(0);
                            let (uid_opt, is_now_hidden) = {
                                let mut s = state_for_tray.lock().unwrap();
                                if let Some(uid) = s.tray_user_ids.get(idx).cloned() {
                                    let was_hidden = s.hidden_users.contains(&uid);
                                    if was_hidden { s.hidden_users.remove(&uid); }
                                    else { s.hidden_users.insert(uid.clone()); }
                                    (Some(uid), !was_hidden)
                                } else { (None, false) }
                            };
                            if let Some(uid) = uid_opt {
                                if let Some(w) = app.get_webview_window(&format!("char_{}", uid)) {
                                    if is_now_hidden { let _ = w.hide(); }
                                    else { let _ = w.show(); }
                                }
                            }
                            rebuild_tray(app, &state_for_tray);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
                let _ = w.emit("open-chat", ());
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            show_main_window,
            quit_app,
            enter_overlay,
            leave_overlay,
            sync_char_windows,
            update_overlay_users,
            store_users_json,
            get_users_json,
            get_user_for_window,
            get_active_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { has_visible_windows, .. } = &_event {
                if !has_visible_windows {
                    if let Some(w) = _app.get_webview_window("main") {
                        let _ = w.show();
                        let _ = w.set_focus();
                        let _ = w.emit("open-chat", ());
                    }
                }
            }
        });
}
