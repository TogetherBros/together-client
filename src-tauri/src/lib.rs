use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{Emitter, Manager};

const TRAY_ID: &str = "main";

// ── Shared state ──────────────────────────────────────────────────────────────

struct OverlayState {
    character_zones: Vec<[i32; 4]>,
    user_ids: Vec<String>,
    is_dragging: bool,
    drag_user_idx: usize,
    last_drag_pos: (i32, i32),
    drag_enabled: bool,
    character_size: u8,          // 1=small 2=medium 3=large
    hidden_users: HashSet<String>,
    tray_user_ids: Vec<String>,
    tray_user_labels: Vec<String>,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            character_zones: Vec::new(),
            user_ids: Vec::new(),
            is_dragging: false,
            drag_user_idx: 0,
            last_drag_pos: (0, 0),
            drag_enabled: true,
            character_size: 2,
            hidden_users: HashSet::new(),
            tray_user_ids: Vec::new(),
            tray_user_labels: Vec::new(),
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

// ── Tray rebuild ──────────────────────────────────────────────────────────────

fn rebuild_tray(app: &tauri::AppHandle, state: &SharedState) {
    use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};

    let (drag_enabled, char_size, hidden_users, user_ids, user_labels) = {
        let s = state.lock().unwrap();
        (s.drag_enabled, s.character_size, s.hidden_users.clone(),
         s.tray_user_ids.clone(), s.tray_user_labels.clone())
    };

    let Ok(show)  = MenuItem::with_id(app, "show",  "채팅창 키기",  true, None::<&str>) else { return };
    let Ok(sep0)  = PredefinedMenuItem::separator(app) else { return };
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
        if let Ok(menu) = Menu::with_items(app, &[&show, &sep0, &drag, &s_sub, &v_sub, &sep1, &reset, &leave, &sep2, &quit]) {
            let _ = tray.set_menu(Some(menu));
        }
    } else if let Ok(menu) = Menu::with_items(app, &[&show, &sep0, &drag, &s_sub, &sep1, &reset, &leave, &sep2, &quit]) {
        let _ = tray.set_menu(Some(menu));
    }
}

// ── Cursor position ───────────────────────────────────────────────────────────

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

#[cfg(target_os = "macos")]
fn get_cursor_pos() -> (i32, i32) {
    use objc2_app_kit::NSEvent;
    #[repr(C)] struct CgPoint { x: f64, y: f64 }
    #[repr(C)] struct CgSize  { width: f64, height: f64 }
    #[repr(C)] struct CgRect  { origin: CgPoint, size: CgSize }
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGMainDisplayID() -> u32;
        fn CGDisplayBounds(display: u32) -> CgRect;
        fn CGDisplayPixelsHigh(display: u32) -> usize;
    }
    unsafe {
        let display = CGMainDisplayID();
        let bounds  = CGDisplayBounds(display);
        let scale   = if bounds.size.height > 0.0 {
            CGDisplayPixelsHigh(display) as f64 / bounds.size.height
        } else { 1.0 };
        let loc = NSEvent::mouseLocation();
        let x = ((loc.x - bounds.origin.x) * scale) as i32;
        let y = ((bounds.size.height - (loc.y - bounds.origin.y)) * scale) as i32;
        (x, y)
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn get_cursor_pos() -> (i32, i32) { (0, 0) }

// ── Left mouse button state ───────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn is_lmb_down() -> bool {
    use winapi::um::winuser::GetAsyncKeyState;
    unsafe { (GetAsyncKeyState(0x01) as u16 & 0x8000) != 0 }
}

#[cfg(target_os = "macos")]
fn is_lmb_down() -> bool {
    use objc2_app_kit::NSEvent;
    // pressedMouseButtons() 비트0 = 좌클릭. CGEventSourceButtonState보다 신뢰성 높음
    unsafe { NSEvent::pressedMouseButtons() & 1 != 0 }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn is_lmb_down() -> bool { false }

// ── System cursor (Windows only) ──────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn set_drag_cursor() {
    use winapi::um::winuser::{CopyIcon, LoadCursorW, SetSystemCursor};
    const IDC_HAND: usize = 32649;
    const OCR_NORMAL: u32 = 32512;
    unsafe {
        let hand = LoadCursorW(std::ptr::null_mut(), IDC_HAND as *const u16);
        if hand.is_null() { return; }
        let copy = CopyIcon(hand as _) as winapi::shared::windef::HCURSOR;
        if !copy.is_null() { SetSystemCursor(copy, OCR_NORMAL); }
    }
}
#[cfg(not(target_os = "windows"))]
fn set_drag_cursor() {}

#[cfg(target_os = "windows")]
fn restore_drag_cursor() {
    use winapi::um::winuser::SystemParametersInfoW;
    const SPI_SETCURSORS: u32 = 0x0057;
    unsafe { SystemParametersInfoW(SPI_SETCURSORS, 0, std::ptr::null_mut(), 0); }
}
#[cfg(not(target_os = "windows"))]
fn restore_drag_cursor() {}

// ── Drag monitor ──────────────────────────────────────────────────────────────

fn drag_monitor(app: tauri::AppHandle, state: SharedState) {
    enum Action { None, StartDrag(String), MoveDrag(String, i32, i32), EndDrag(String) }

    loop {
        thread::sleep(Duration::from_millis(16));

        let Some(overlay) = app.get_webview_window("overlay") else { continue };
        if !overlay.is_visible().unwrap_or(false) { continue; }

        let (cx, cy) = get_cursor_pos();
        let lmb = is_lmb_down();

        let action = {
            let mut s = state.lock().unwrap();

            if !s.drag_enabled {
                if s.is_dragging && !lmb {
                    let uid = s.user_ids.get(s.drag_user_idx).cloned().unwrap_or_default();
                    s.is_dragging = false;
                    Action::EndDrag(uid)
                } else {
                    Action::None
                }
            } else if !s.is_dragging && lmb {
                let idx = s.character_zones.iter().position(|z| {
                    cx >= z[0] && cx < z[0] + z[2] && cy >= z[1] && cy < z[1] + z[3]
                });
                if let Some(i) = idx {
                    let uid = s.user_ids.get(i).cloned().unwrap_or_default();
                    s.is_dragging = true;
                    s.drag_user_idx = i;
                    s.last_drag_pos = (cx, cy);
                    Action::StartDrag(uid)
                } else { Action::None }
            } else if s.is_dragging {
                if lmb {
                    let dx = cx - s.last_drag_pos.0;
                    let dy = cy - s.last_drag_pos.1;
                    if dx != 0 || dy != 0 {
                        let uid = s.user_ids.get(s.drag_user_idx).cloned().unwrap_or_default();
                        s.last_drag_pos = (cx, cy);
                        Action::MoveDrag(uid, dx, dy)
                    } else { Action::None }
                } else {
                    let uid = s.user_ids.get(s.drag_user_idx).cloned().unwrap_or_default();
                    s.is_dragging = false;
                    Action::EndDrag(uid)
                }
            } else { Action::None }
        };

        match action {
            Action::StartDrag(uid) => { set_drag_cursor(); let _ = overlay.emit("drag-start", uid); }
            Action::MoveDrag(uid, dx, dy) => { let _ = overlay.emit("drag-move", (uid, dx, dy)); }
            Action::EndDrag(uid) => { restore_drag_cursor(); let _ = overlay.emit("drag-end", uid); }
            Action::None => {}
        }
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn show_main_window(app: tauri::AppHandle) { restore_window(&app); }

#[tauri::command]
fn quit_app(app: tauri::AppHandle) { app.exit(0); }

#[tauri::command]
fn enter_overlay(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
) -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    { let _ = (app, state); return Ok(false); }

    #[cfg(not(target_os = "linux"))]
    {
        if let Some(w) = app.get_webview_window("main") { let _ = w.hide(); }
        if let Some(w) = app.get_webview_window("overlay") {
            let _ = w.set_ignore_cursor_events(true);
            let _ = w.show();
        }
        let (char_size, hidden_vec) = {
            let mut s = state.lock().unwrap();
            s.character_zones.clear();
            s.user_ids.clear();
            s.is_dragging = false;
            let hv: Vec<String> = s.hidden_users.iter().cloned().collect();
            (s.character_size, hv)
        };
        if let Some(w) = app.get_webview_window("overlay") {
            let _ = w.emit("character-size-changed", char_size);
            let _ = w.emit("hidden-users-changed", hidden_vec);
        }
        Ok(true)
    }
}

#[tauri::command]
fn leave_overlay(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("overlay") { let _ = w.hide(); }
    restore_window(&app);
    {
        let mut s = state.lock().unwrap();
        s.character_zones.clear();
        s.user_ids.clear();
        s.is_dragging = false;
        s.tray_user_ids.clear();
        s.tray_user_labels.clear();
    }
    rebuild_tray(&app, &state);
    Ok(())
}

#[tauri::command]
fn update_character_zones(
    zones: Vec<Vec<i32>>,
    user_ids: Vec<String>,
    state: tauri::State<'_, SharedState>,
) {
    let mut s = state.lock().unwrap();
    s.character_zones = zones.into_iter()
        .filter_map(|z| if z.len() == 4 { Some([z[0], z[1], z[2], z[3]]) } else { None })
        .collect();
    s.user_ids = user_ids;
}

#[tauri::command]
fn update_overlay_users(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    user_ids: Vec<String>,
    user_labels: Vec<String>,
) {
    {
        let mut s = state.lock().unwrap();
        s.hidden_users.retain(|id| user_ids.contains(id));
        s.tray_user_ids = user_ids;
        s.tray_user_labels = user_labels;
    }
    rebuild_tray(&app, &state);
}

#[tauri::command]
fn get_active_app() -> String { get_active_app_internal() }

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

// ── App setup ─────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shared_state: SharedState = Arc::new(Mutex::new(OverlayState::default()));

    tauri::Builder::default()
        .manage(shared_state.clone())
        .setup(move |app| {
            use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
            use tauri::tray::TrayIconBuilder;

            #[cfg(not(target_os = "linux"))]
            {
                let app_handle = app.handle().clone();
                let state_for_thread = shared_state.clone();
                thread::spawn(move || drag_monitor(app_handle, state_for_thread));
            }

            #[cfg(not(target_os = "linux"))]
            tauri::WebviewWindowBuilder::new(app, "overlay", tauri::WebviewUrl::App("index.html".into()))
                .decorations(false).transparent(true).always_on_top(true)
                .skip_taskbar(true).maximized(true).resizable(false)
                .shadow(false).focused(false).visible(false)
                .build()?;

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
            let menu  = Menu::with_items(app, &[&show, &sep0, &drag, &s_sub, &sep1, &reset, &leave, &sep2, &quit])?;

            let state_for_tray = shared_state.clone();
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.emit("open-chat", ());
                            }
                        }
                        "toggle-drag" => {
                            { state_for_tray.lock().unwrap().drag_enabled ^= true; }
                            rebuild_tray(app, &state_for_tray);
                        }
                        id @ ("size-1" | "size-2" | "size-3") => {
                            let size: u8 = id.trim_start_matches("size-").parse().unwrap_or(2);
                            { state_for_tray.lock().unwrap().character_size = size; }
                            if let Some(w) = app.get_webview_window("overlay") {
                                let _ = w.emit("character-size-changed", size);
                            }
                            rebuild_tray(app, &state_for_tray);
                        }
                        "reset" => {
                            if let Some(w) = app.get_webview_window("overlay") {
                                let _ = w.emit("reset-positions", ());
                            }
                        }
                        "leave" => {
                            if let Some(w) = app.get_webview_window("overlay") { let _ = w.hide(); }
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.emit("leave-overlay", ());
                            }
                        }
                        "quit" => app.exit(0),
                        id if id.starts_with("vis-") => {
                            let idx: usize = id.trim_start_matches("vis-").parse().unwrap_or(0);
                            let hidden_vec = {
                                let mut s = state_for_tray.lock().unwrap();
                                if let Some(uid) = s.tray_user_ids.get(idx).cloned() {
                                    if s.hidden_users.contains(&uid) {
                                        s.hidden_users.remove(&uid);
                                    } else {
                                        s.hidden_users.insert(uid);
                                    }
                                }
                                s.hidden_users.iter().cloned().collect::<Vec<_>>()
                            };
                            if let Some(w) = app.get_webview_window("overlay") {
                                let _ = w.emit("hidden-users-changed", hidden_vec);
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
                let _ = w.emit("open-chat", ());
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            show_main_window,
            quit_app,
            enter_overlay,
            leave_overlay,
            update_character_zones,
            update_overlay_users,
            get_active_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { has_visible_windows, .. } = &_event {
                if !has_visible_windows {
                    if let Some(w) = _app.get_webview_window("main") {
                        let _ = w.emit("open-chat", ());
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
        });
}
