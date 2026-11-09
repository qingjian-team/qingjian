//! [`Strip`]：一条贴边的分层窗口；每帧的像素由 [`Edge`] 算。

use std::mem::size_of;

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HBITMAP, HDC,
    SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, SW_SHOWNOACTIVATE, ShowWindow,
    ULW_ALPHA, UpdateLayeredWindow, WINDOW_EX_STYLE, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::w;

use crate::system::neon::{BLUE, Edge, PixelOrder};

pub(super) struct Strip {
    window: HWND,

    context: HDC,

    bitmap: HBITMAP,

    /// DIB 的像素：预乘的 BGRA，从上到下。
    pixels: *mut u8,

    origin: POINT,

    edge: Edge,
}

unsafe extern "system" fn procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: 原样转给系统的默认处理。
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

impl Strip {
    pub(super) fn create(edge: Edge, at: (i32, i32)) -> Option<Self> {
        let size = edge.size;
        let instance = unsafe { GetModuleHandleW(None) }.ok()?;
        let class = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance.into(),
            lpszClassName: w!("QingjianClipboardNeon"),
            ..WNDCLASSW::default()
        };
        // 重复注册会失败，不要紧
        unsafe { RegisterClassW(&class) };
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(
                    WS_EX_LAYERED.0
                        | WS_EX_TRANSPARENT.0
                        | WS_EX_TOPMOST.0
                        | WS_EX_TOOLWINDOW.0
                        | WS_EX_NOACTIVATE.0,
                ),
                w!("QingjianClipboardNeon"),
                w!(""),
                WS_POPUP,
                at.0,
                at.1,
                size.0,
                size.1,
                None,
                None,
                Some(instance.into()),
                None,
            )
        }
        .ok()?;
        let context = unsafe { CreateCompatibleDC(None) };
        let header = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size.0,
            biHeight: -size.1,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..BITMAPINFOHEADER::default()
        };
        let info = BITMAPINFO {
            bmiHeader: header,
            ..BITMAPINFO::default()
        };
        let mut pixels = std::ptr::null_mut();
        let bitmap =
            unsafe { CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut pixels, None, 0) }.ok()?;
        unsafe { SelectObject(context, bitmap.into()) };
        let strip = Self {
            window,
            context,
            bitmap,
            pixels: pixels.cast(),
            origin: POINT { x: at.0, y: at.1 },
            edge,
        };
        strip.draw(BLUE, 0.0, 0.0);
        let _ = unsafe { ShowWindow(window, SW_SHOWNOACTIVATE) };
        Some(strip)
    }

    pub(super) fn draw(&self, color: [f32; 3], time: f32, strength: f32) {
        let (width, height) = self.edge.size;
        // SAFETY: pixels 是这条窗口自己的 DIB，大小就是宽 × 高 × 4，只有这个线程写。
        let pixels =
            unsafe { std::slice::from_raw_parts_mut(self.pixels, (width * height * 4) as usize) };
        self.edge
            .paint(pixels, PixelOrder::Bgra, color, time, strength);
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
            ..BLENDFUNCTION::default()
        };
        let source = POINT::default();
        let _ = unsafe {
            UpdateLayeredWindow(
                self.window,
                None,
                Some(&self.origin),
                Some(&SIZE {
                    cx: width,
                    cy: height,
                }),
                Some(self.context),
                Some(&source),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )
        };
    }
}

impl Drop for Strip {
    fn drop(&mut self) {
        // SAFETY: 这些句柄都是这条窗口自己建的，只释放一次。
        unsafe {
            let _ = DestroyWindow(self.window);
            let _ = DeleteDC(self.context);
            let _ = DeleteObject(self.bitmap.into());
        }
    }
}
