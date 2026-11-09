//! 已注册的青简的端到端检查：开一个 RichEdit 窗口，只把本进程切到青简，用 SendInput 敲一段脚本再打印文本。
//!
//! `cargo run -p qingjian-windows-tsf --example host -- [tsf|imm][+uiless] "<脚本>"`
//! `uiless` 让宿主像游戏一样自己取候选（`ITfUIElementSink` 回 `bShow = FALSE`）并打印出来。
//! 每敲一键后青简候选窗（Server 进程里）显示 / 收起变了就打印一行。
//! 脚本按美式键盘敲；`{名字}` 是特殊键：enter、esc、bs、space、pgdn、shift（单按 Shift）、wait（等 300 ms）。
//! 按键发给前台窗口，本窗口失去焦点就停。Windows 缺省按整个桌面切输入法，结束时切回原来的那个；
//! 中途没跑完时用 `host restore {clsid} {profile}` 切回指定的（zh-CN），切完打印当前的。需要 Server 已在跑。

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
fn main() -> windows::core::Result<()> {
    host::run()
}

#[cfg(windows)]
mod host {
    use std::time::{Duration, Instant};

    use windows::Win32::Foundation::{FALSE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    };
    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, LoadLibraryW};
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        HKL, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
        MAPVK_VK_TO_VSC, MapVirtualKeyW, SendInput, SetFocus, VIRTUAL_KEY,
    };
    use windows::Win32::UI::TextServices::{
        CLSID_TF_InputProcessorProfiles, CLSID_TF_ThreadMgr, GUID_TFCAT_TIP_KEYBOARD,
        ITfCandidateListUIElement, ITfInputProcessorProfileMgr, ITfSource, ITfThreadMgr,
        ITfUIElementMgr, ITfUIElementSink, ITfUIElementSink_Impl, TF_INPUTPROCESSORPROFILE,
        TF_IPPMF_DONTCARECURRENTINPUTLANGUAGE, TF_IPPMF_FORPROCESS, TF_IPPMF_FORSESSION,
        TF_PROFILETYPE_INPUTPROCESSOR,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FindWindowW,
        GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible, MSG, PM_REMOVE, PeekMessageW, RegisterClassW, SendMessageW,
        SetForegroundWindow, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW, WS_CHILD,
        WS_EX_TOPMOST, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
    };
    use windows::core::{BOOL, GUID, Interface, Result, w};

    /// 与 `src/com/mod.rs` 的 `CLSID_QINGJIAN` / `GUID_PROFILE` 同步。
    const CLSID: GUID = GUID::from_u128(0x4fdca82d_e923_49bf_9e75_bb906b93b8bb);
    const PROFILE: GUID = GUID::from_u128(0x8119f8e0_cf81_423b_9189_c0d7374324b3);
    const LANGID_ZH_CN: u16 = 0x0804;

    /// 像自己画候选的游戏那样取候选，每次更新打印一行。
    #[windows_core::implement(ITfUIElementSink)]
    struct Sink(ITfUIElementMgr);

    impl ITfUIElementSink_Impl for Sink_Impl {
        fn BeginUIElement(&self, _id: u32, show: *mut BOOL) -> Result<()> {
            unsafe { *show = FALSE };
            println!("  ui-less: begin");
            Ok(())
        }

        fn UpdateUIElement(&self, id: u32) -> Result<()> {
            let list: ITfCandidateListUIElement = unsafe { self.0.GetUIElement(id)?.cast()? };
            let (count, selection, page) = unsafe {
                (
                    list.GetCount()?,
                    list.GetSelection()?,
                    list.GetCurrentPage()?,
                )
            };
            let items: Vec<String> = (0..count)
                .map(|i| {
                    unsafe { list.GetString(i) }.map_or_else(|_| "?".into(), |s| s.to_string())
                })
                .collect();
            println!(
                "  ui-less: page {page}, selected {selection}, {count}: {}",
                items.join(" ")
            );
            Ok(())
        }

        fn EndUIElement(&self, _id: u32) -> Result<()> {
            println!("  ui-less: end");
            Ok(())
        }
    }

    pub fn run() -> Result<()> {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let mode = args.first().map_or("", String::as_str);
        let script = args.get(1).cloned().unwrap_or_default();
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
            let profiles: ITfInputProcessorProfileMgr =
                CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
            if mode == "restore" {
                let guid = |i: usize| {
                    GUID::try_from(args[i].trim_matches(|c| c == '{' || c == '}')).unwrap()
                };
                profiles.ActivateProfile(
                    TF_PROFILETYPE_INPUTPROCESSOR,
                    LANGID_ZH_CN,
                    &guid(1),
                    &guid(2),
                    HKL::default(),
                    TF_IPPMF_FORSESSION,
                )?;
            }
            let mut previous = TF_INPUTPROCESSORPROFILE::default();
            profiles.GetActiveProfile(&GUID_TFCAT_TIP_KEYBOARD, &mut previous)?;
            println!(
                "before: {{{:?}}} {{{:?}}}",
                previous.clsid, previous.guidProfile
            );
            if mode == "restore" {
                return Ok(());
            }
            LoadLibraryW(w!("msftedit.dll"))?;
            let instance = GetModuleHandleW(None)?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: instance.into(),
                lpszClassName: w!("QingjianHost"),
                ..Default::default()
            };
            RegisterClassW(&class);
            let main = CreateWindowExW(
                WS_EX_TOPMOST,
                w!("QingjianHost"),
                w!("Qingjian host"),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                200,
                200,
                700,
                300,
                None,
                None,
                Some(instance.into()),
                None,
            )?;
            // ES_MULTILINE
            let edit = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("RICHEDIT50W"),
                w!(""),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(4),
                10,
                10,
                660,
                240,
                Some(main),
                None,
                Some(instance.into()),
                None,
            )?;
            if mode.starts_with("tsf") {
                // EM_SETEDITSTYLE + SES_USECTF：RichEdit 直接走 TSF，不经 IMM32
                SendMessageW(edit, 0x4CC, Some(WPARAM(0x10000)), Some(LPARAM(0x10000)));
            }
            let foreground = GetWindowThreadProcessId(GetForegroundWindow(), None);
            let _ = AttachThreadInput(GetCurrentThreadId(), foreground, true);
            let _ = SetForegroundWindow(main);
            let _ = AttachThreadInput(GetCurrentThreadId(), foreground, false);
            let _ = SetFocus(Some(edit));
            pump(300);
            profiles.ActivateProfile(
                TF_PROFILETYPE_INPUTPROCESSOR,
                LANGID_ZH_CN,
                &CLSID,
                &PROFILE,
                HKL::default(),
                TF_IPPMF_FORPROCESS | TF_IPPMF_DONTCARECURRENTINPUTLANGUAGE,
            )?;
            pump(300);
            if mode.contains("uiless") {
                let threads: ITfThreadMgr =
                    CoCreateInstance(&CLSID_TF_ThreadMgr, None, CLSCTX_INPROC_SERVER)?;
                let sink: ITfUIElementSink = Sink(threads.cast()?).into();
                threads
                    .cast::<ITfSource>()?
                    .AdviseSink(&ITfUIElementSink::IID, &sink)?;
            }
            let mut active = TF_INPUTPROCESSORPROFILE::default();
            profiles.GetActiveProfile(&GUID_TFCAT_TIP_KEYBOARD, &mut active)?;
            println!(
                "active: {}",
                if active.clsid == CLSID {
                    "青简"
                } else {
                    "别的输入法"
                }
            );
            let mut window_shown = false;
            let mut rest = script.as_str();
            while let Some(c) = rest.chars().next() {
                if GetForegroundWindow() != main {
                    println!("stopped: 窗口失去了焦点");
                    break;
                }
                if c == '{' {
                    let end = rest.find('}').expect("{ 没有配对的 }");
                    match &rest[1..end] {
                        "enter" => press(&[], 0x0D),
                        "esc" => press(&[], 0x1B),
                        "bs" => press(&[], 0x08),
                        "space" => press(&[], 0x20),
                        "pgdn" => press(&[], 0x22),
                        "shift" => press(&[], 0x10),
                        "wait" => pump(300),
                        key => panic!("不认识的键 {key}"),
                    }
                    rest = &rest[end + 1..];
                } else {
                    let (key, shift) = vk(c);
                    press(if shift { &[0x10] } else { &[] }, key);
                    rest = &rest[c.len_utf8()..];
                }
                let shown = FindWindowW(w!("QingjianCandidateWindow"), None)
                    .is_ok_and(|window| IsWindowVisible(window).as_bool());
                if shown != window_shown {
                    window_shown = shown;
                    println!("  青简候选窗: {}", if shown { "显示" } else { "收起" });
                }
            }
            pump(200);
            println!("text: {:?}", text(edit));
            let _ = DestroyWindow(main);
            profiles.ActivateProfile(
                previous.dwProfileType,
                previous.langid,
                &previous.clsid,
                &previous.guidProfile,
                previous.hkl,
                TF_IPPMF_FORSESSION,
            )?;
        }
        Ok(())
    }

    fn pump(ms: u64) {
        let end = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < end {
            unsafe {
                let mut message = MSG::default();
                while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn send(vk: u16, up: bool) {
        let ki = KEYBDINPUT {
            wVk: VIRTUAL_KEY(vk),
            wScan: unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) } as u16,
            dwFlags: if up {
                KEYEVENTF_KEYUP
            } else {
                KEYBD_EVENT_FLAGS(0)
            },
            ..Default::default()
        };
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki },
        };
        unsafe { SendInput(&[input], size_of::<INPUT>() as i32) };
        pump(15);
    }

    /// 按住 `mods` 按一下 `vk`。
    fn press(mods: &[u16], vk: u16) {
        mods.iter().for_each(|&m| send(m, false));
        send(vk, false);
        send(vk, true);
        mods.iter().rev().for_each(|&m| send(m, true));
        // 留时间给 Server 回包与轮询（80 ms 一拍）
        pump(120);
    }

    /// 美式键盘上打出 `c` 的键与要不要按 Shift。
    fn vk(c: char) -> (u16, bool) {
        let rows: [(u16, &str, &str); 3] = [
            (0x30, "0123456789", ")!@#$%^&*("),
            (0xBA, ";=,-./`", ":+<_>?~"),
            (0xDB, "[\\]'", "{|}\""),
        ];
        match c {
            'a'..='z' => (c as u16 - 32, false),
            'A'..='Z' => (c as u16, true),
            ' ' => (0x20, false),
            _ => rows
                .iter()
                .find_map(|(base, plain, shifted)| {
                    let at = |keys: &str| keys.chars().position(|x| x == c);
                    at(plain)
                        .map(|i| (base + i as u16, false))
                        .or_else(|| at(shifted).map(|i| (base + i as u16, true)))
                })
                .unwrap_or_else(|| panic!("美式键盘打不出 {c:?}")),
        }
    }

    extern "system" fn wndproc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe { DefWindowProcW(window, message, wparam, lparam) }
    }

    fn text(edit: HWND) -> String {
        unsafe {
            let mut buffer = vec![0u16; GetWindowTextLengthW(edit) as usize + 1];
            let n = GetWindowTextW(edit, &mut buffer) as usize;
            String::from_utf16_lossy(&buffer[..n])
        }
    }
}
