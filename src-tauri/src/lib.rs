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

#[cfg(target_os = "macos")]
fn get_cursor_pos() -> (i32, i32) {
    #[repr(C)] struct CGPoint { x: f64, y: f64 }
    #[repr(C)] struct CGSize  { width: f64, height: f64 }
    #[repr(C)] struct CGRect  { origin: CGPoint, size: CGSize }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGMainDisplayID() -> u32;
        fn CGDisplayBounds(display: u32) -> CGRect;
        fn CGDisplayPixelsHigh(display: u32) -> usize;
        fn CGEventCreate(source: *const core::ffi::c_void) -> *mut core::ffi::c_void;
        fn CGEventGetLocation(event: *const core::ffi::c_void) -> CGPoint;
        fn CFRelease(cf: *const core::ffi::c_void);
    }

    unsafe {
        let display   = CGMainDisplayID();
        let bounds    = CGDisplayBounds(display);
        // scale = physical pixels / logical points (e.g. 2.0 on Retina)
        let scale     = if bounds.size.height > 0.0 {
            CGDisplayPixelsHigh(display) as f64 / bounds.size.height
        } else { 1.0 };

        let e = CGEventCreate(core::ptr::null());
        if e.is_null() { return (0, 0); }
        let p = CGEventGetLocation(e);
        CFRelease(e);

        // macOS CG: bottom-left origin → convert to top-left physical pixels
        // to match window.screenX/Y * devicePixelRatio used in JS zones
        let x = ((p.x - bounds.origin.x) * scale) as i32;
        let y = ((bounds.size.height - (p.y - bounds.origin.y)) * scale) as i32;
        (x, y)
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn get_cursor_pos() -> (i32, i32) { (0, 0) }

// ── Left mouse button state (Windows) ────────────────────────────────────────

#[cfg(target_os = "windows")]
fn is_lmb_down() -> bool {
    use winapi::um::winuser::GetAsyncKeyState;
    unsafe { (GetAsyncKeyState(0x01) as u16 & 0x8000) != 0 }
}

#[cfg(target_os = "macos")]
fn is_lmb_down() -> bool {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventSourceButtonState(stateID: i32, button: u32) -> u8;
    }
    unsafe { CGEventSourceButtonState(1, 0) != 0 }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn is_lmb_down() -> bool { false }


// ── System cursor replacement (Windows) ──────────────────────────────────────
// passthrough 토글 없이 시스템 전체 화살표 커서를 손 모양으로 교체.
// SetSystemCursor는 커서 핸들 소유권을 가져가므로 반드시 CopyIcon으로 복사본 전달.
// 드래그 종료 시 SPI_SETCURSORS로 레지스트리 기본값으로 복원.

#[cfg(target_os = "windows")]
fn set_drag_cursor() {
    use winapi::um::winuser::{CopyIcon, LoadCursorW, SetSystemCursor};
    const IDC_HAND:   usize = 32649;
    const OCR_NORMAL: u32   = 32512;
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
// 오버레이는 항상 passthrough. Rust가 OS 레벨에서 마우스 버튼+위치를 폴링해
// 드래그를 감지하고 물리 픽셀 델타를 drag-move 이벤트로 오버레이 JS에 전달.

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

        let (action, _in_zone, _is_dragging_now) = {
            let mut s = state.lock().unwrap();

            let in_zone = s.character_zones.iter().any(|z| {
                cx >= z[0] && cx < z[0] + z[2] && cy >= z[1] && cy < z[1] + z[3]
            });

            let action = if !s.is_dragging && lmb {
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
            };

            (action, in_zone, s.is_dragging)
        };


        match action {
            Action::StartDrag(uid) => {
                set_drag_cursor();
                let _ = overlay.emit("drag-start", uid);
            }
            Action::MoveDrag(uid, dx, dy) => { let _ = overlay.emit("drag-move", (uid, dx, dy)); }
            Action::EndDrag(uid) => {
                restore_drag_cursor();
                let _ = overlay.emit("drag-end", uid);
            }
            Action::None => {}
        }
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn show_main_window(app: tauri::AppHandle) {
    restore_window(&app);
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn enter_overlay(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
) -> Result<bool, String> {
    // Linux는 오버레이 미지원 → 채팅 화면 유지 (false 반환)
    #[cfg(target_os = "linux")]
    {
        let _ = app;
        let _ = state;
        return Ok(false);
    }

    #[cfg(not(target_os = "linux"))]
    {
        if let Some(w) = app.get_webview_window("main") { let _ = w.hide(); }
        if let Some(w) = app.get_webview_window("overlay") {
            let _ = w.set_ignore_cursor_events(true);
            let _ = w.show();
        }
        let mut s = state.lock().unwrap();
        s.character_zones.clear();
        s.user_ids.clear();
        s.is_dragging = false;
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
        "safari" | "chrome" | "firefox" | "arc" | "whale" | "opera" | "brave" => {
            return format!("{} 실행 중", name);
        }
        "xcode" => "Xcode 작업 중",
        "visual studio code" | "code" => "VS Code 작업 중",
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
        "terminal" | "iterm2" | "warp" => "터미널 중",
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

#[cfg(target_os = "windows")]
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

            #[cfg(not(target_os = "linux"))]
            {
                let app_handle = app.handle().clone();
                let state_for_thread = shared_state.clone();
                thread::spawn(move || drag_monitor(app_handle, state_for_thread));
            }

            // Linux는 Wayland/X11 투명 오버레이 미지원 → 오버레이 창 생성 생략
            #[cfg(not(target_os = "linux"))]
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

            #[cfg(debug_assertions)]
            if let Some(w) = app.get_webview_window("main") {
                w.open_devtools();
            }

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

            use tauri::menu::PredefinedMenuItem;

            let show  = MenuItem::with_id(app, "show",  "채팅창 키기", true, None::<&str>)?;
            let leave = MenuItem::with_id(app, "leave", "방 나가기",   true, None::<&str>)?;
            let sep   = PredefinedMenuItem::separator(app)?;
            let quit  = MenuItem::with_id(app, "quit",  "종료하기",    true, None::<&str>)?;
            let menu  = Menu::with_items(app, &[&show, &leave, &sep, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.emit("open-chat", ());
                        }
                    }
                    "leave" => {
                        if let Some(w) = app.get_webview_window("overlay") { let _ = w.hide(); }
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
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // 두 번째 인스턴스 실행 시 → 기존 창에 open-chat 신호
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
            get_active_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // macOS 독 아이콘 클릭 시 (창이 숨겨진 상태)
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { has_visible_windows, .. } = &event {
                if !has_visible_windows {
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.emit("open-chat", ());
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
        });
}
