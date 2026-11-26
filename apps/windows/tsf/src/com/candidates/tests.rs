use super::{CandidateWindow, Style, is_shell_process};
use qingjian_platform::protocol::InputSettings;
use windows::Win32::Foundation::RECT as WinRect;
use windows::core::w;

#[test]
fn shells_are_matched_by_exact_lowercase_name() {
    assert!(is_shell_process("searchhost.exe"));
    assert!(is_shell_process("startmenuexperiencehost.exe"));
    assert!(!is_shell_process("explorer.exe"));
    // 大小写不匹配是调用方的事：`classify` 先转小写，这里不做模糊匹配，免得把 `searchhost.exe.bak` 之类算进去。
    assert!(!is_shell_process("SearchHost.exe"));
}

#[test]
fn style_reads_the_settings_the_server_sends() {
    let input = InputSettings {
        candidate_font_size: qingjian_platform::FontSize(20.0),
        annotation_font_size: qingjian_platform::FontSize(12.0),
        candidate_weight: Some(500),
        reverse_above: true,
        ..InputSettings::default()
    };
    let style = Style::from_settings(&input);
    assert_eq!(style.candidate, 20.0);
    assert_eq!(style.annotation, 12.0);
    assert_eq!(style.weight, Some(500));
    assert!(style.reverse_above);
}

/// 0 字号是「用内置的」，进程内候选窗按缺省字号画。
#[test]
fn zero_sizes_mean_the_builtin_default() {
    let style = Style::from_settings(&InputSettings::default());
    assert_eq!(style.candidate, 0.0);
    assert_eq!(style.annotation, 0.0);
}

/// 把屏幕上一块区域截下来存成 PNG（冒烟测试用）。分层窗能不能被抓到取决于 DWM，抓不到也只是背景。
fn capture(hwnd: windows::Win32::Foundation::HWND, path: &std::path::Path) {
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
        DeleteDC, DeleteObject, GetDC, SelectObject,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let mut rect = WinRect::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.expect("窗口矩形");
    let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);
    let screen = unsafe { GetDC(None) };
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits = core::ptr::null_mut();
    let dib = unsafe { CreateDIBSection(Some(screen), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) }
        .expect("截图 DIB");
    let mem = unsafe { CreateCompatibleDC(Some(screen)) };
    let previous = unsafe { SelectObject(mem, dib.into()) };
    // 分层窗由 DWM 合成，桌面 DC 上 BitBlt 抓不到它；PrintWindow 让窗口把自己的位图交出来
    // （这个绑定在 windows crate 里落在 Storage::Xps 模块下）。
    let printed = unsafe {
        windows::Win32::Storage::Xps::PrintWindow(
            hwnd,
            mem,
            windows::Win32::Storage::Xps::PRINT_WINDOW_FLAGS(2),
        )
    };
    println!("PrintWindow rendered={}", printed.as_bool());
    let pixels = unsafe { std::slice::from_raw_parts(bits as *const u8, (w * h * 4) as usize) };
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for pixel in pixels.as_chunks::<4>().0 {
        rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
    }
    unsafe {
        let _ = SelectObject(mem, previous);
        let _ = DeleteObject(dib.into());
        let _ = DeleteDC(mem);
        windows::Win32::Graphics::Gdi::ReleaseDC(None, screen);
    }
    let file = std::fs::File::create(path).expect("截图文件");
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        u32::try_from(w).unwrap(),
        u32::try_from(h).unwrap(),
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(&rgba))
        .expect("写 PNG");
}

/// 真建一扇窗、真走一遍「建字体 → 量尺寸 → GDI 落笔 → 贴分层位图 → 显示」：
/// 验证字体、绘制、贴图与摆位都不炸，并落下截图供人工看一眼。需要桌面会话，默认跳过。
#[test]
#[ignore = "需要桌面会话：真的建窗口、画位图"]
fn window_renders_a_candidate_frame() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, GetWindowRect, IsWindowVisible, WINDOW_EX_STYLE,
        WINDOW_STYLE,
    };

    use qingjian_core::{Candidate, CandidateKind, CandidateList};
    use qingjian_platform::protocol::{Frame, PreeditKind, PreeditSegment};

    let host = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            w!("host"),
            WINDOW_STYLE(0),
            0,
            0,
            200,
            100,
            None,
            None,
            None,
            None,
        )
    }
    .expect("测试宿主窗口");
    let client: crate::com::service::SharedClient = Rc::new(RefCell::new(None));
    let created = std::time::Instant::now();
    let window = CandidateWindow::new(host, client).expect("候选窗");
    let create_ms = created.elapsed().as_millis();
    let frame = Frame {
        preedit: vec![PreeditSegment {
            text: "ni'hao".to_owned(),
            kind: PreeditKind::Typed,
        }],
        candidates: CandidateList {
            items: vec![
                Candidate {
                    text: "你好".to_owned(),
                    kind: CandidateKind::Chinese,
                    syllables: vec!["ni".to_owned(), "hao".to_owned()],
                    reading: None,
                    translation: None,
                    aux_code: None,
                },
                Candidate {
                    text: "拟好".to_owned(),
                    kind: CandidateKind::Chinese,
                    syllables: vec!["ni".to_owned(), "hao".to_owned()],
                    reading: None,
                    translation: None,
                    aux_code: None,
                },
            ],
        },
        page_count: 1,
        ..Frame::default()
    };
    let first = std::time::Instant::now();
    window.show(
        &frame,
        WinRect {
            left: 120,
            top: 120,
            right: 200,
            bottom: 140,
        },
    );
    let first_ms = first.elapsed().as_millis();
    let warm = std::time::Instant::now();
    window.show(
        &frame,
        WinRect {
            left: 120,
            top: 120,
            right: 200,
            bottom: 140,
        },
    );
    let warm_ms = warm.elapsed().as_millis();
    println!(
        "candidate window: create={create_ms}ms first_show={first_ms}ms warm_show={warm_ms}ms"
    );
    let mut rect = WinRect::default();
    unsafe { GetWindowRect(window.hwnd, &mut rect) }.expect("窗口矩形");
    assert!(
        rect.right > rect.left && rect.bottom > rect.top,
        "候选窗没拿到大小"
    );
    assert!(
        unsafe { IsWindowVisible(window.hwnd) }.as_bool(),
        "候选窗没显示"
    );
    // 截图前把窗口提到最前：测试宿主的 owner 是个看不见的窗口，候选窗的 z 序很低，
    // 不置顶就会被前台应用盖住、截到的是别人。只影响这次截图，生产里不置顶。
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
        };
        SetWindowPos(
            window.hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
        .expect("截图前置顶");
    }
    // 截一块屏幕存成 PNG：人工看一眼候选窗是不是真画出来了、字有没有缺。
    std::thread::sleep(std::time::Duration::from_millis(300));
    let path = std::env::temp_dir().join("qingjian-candidate-smoke.png");
    println!(
        "window rect {:?}",
        (rect.left, rect.top, rect.right, rect.bottom)
    );
    capture(window.hwnd, &path);
    assert!(path.exists(), "截图没落盘");
    // 命中区落在内容区里：点第一个候选的中心能认出来（点候选回 Server 全靠它）。
    let point = super::CLICKS.with(|clicks| {
        let clicks = clicks.borrow();
        let clicks = clicks.as_ref().expect("点击状态");
        let region = clicks.hits.first().expect("候选行命中区");
        windows::Win32::Foundation::POINT {
            x: clicks.origin.0 + region.x as i32 + 1,
            y: clicks.origin.1 + region.y as i32 + 1,
        }
    });
    assert!(super::target_at(point).is_some(), "候选行点不中");
    window.hide();
    assert!(
        !unsafe { IsWindowVisible(window.hwnd) }.as_bool(),
        "候选窗收起后还可见"
    );
    drop(window);
    unsafe { DestroyWindow(host) }.expect("销毁测试宿主窗口");
}
