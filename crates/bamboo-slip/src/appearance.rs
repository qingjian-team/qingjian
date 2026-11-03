//! 系统的外观设置：强调色与透明材质。读不到（别的平台、老系统）时使用方退回 [`Accent::FLUENT_BLUE`] 或自己的品牌色。

use crate::color::Color;

/// 一个强调色在两种外观下的取值：同一个颜色在浅底、深底上要用不同明度才看得清。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Accent {
    pub on_light: Color,

    pub on_dark: Color,
}

impl Accent {
    /// Windows 11 默认的蓝。
    pub const FLUENT_BLUE: Self = Self {
        on_light: Color::rgb(0x00, 0x67, 0xC0),
        on_dark: Color::rgb(0x4C, 0xC2, 0xFF),
    };

    /// 用户在系统里选的强调色；每次调用都重新读，用户可能刚改过。
    pub fn system() -> Option<Self> {
        #[cfg(target_os = "windows")]
        {
            read_palette().and_then(|palette| Self::from_palette(&palette))
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    }

    /// 注册表 `AccentPalette`：8 个 RGBA，从最亮排到最暗。系统在浅色下用第 5 个，深色下用第 2 个。
    #[cfg_attr(not(any(target_os = "windows", test)), allow(dead_code))]
    fn from_palette(palette: &[u8]) -> Option<Self> {
        let color = |index: usize| {
            let rgb = palette.get(index * 4..index * 4 + 3)?;
            Some(Color::rgb(rgb[0], rgb[1], rgb[2]))
        };
        Some(Self {
            on_light: color(4)?,
            on_dark: color(1)?,
        })
    }
}

/// 系统支持亚克力，用户也没关「透明效果」。第一个支持的版本是 Windows 11 22H2（22621）。
pub fn system_acrylic() -> bool {
    #[cfg(target_os = "windows")]
    {
        read_build() >= Some(22621) && read_transparency() != Some(0)
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

#[cfg(target_os = "windows")]
fn read_transparency() -> Option<u32> {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    use windows::core::w;

    let mut value = 0u32;
    let mut size = 4u32;
    // SAFETY: 缓冲与长度对应，字符串是编译期的 UTF-16 常量。
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            w!("EnableTransparency"),
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut value).cast()),
            Some(&mut size),
        )
    };
    status.is_ok().then_some(value)
}

#[cfg(target_os = "windows")]
fn read_build() -> Option<u32> {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
    use windows::core::w;

    let mut text = [0u16; 16];
    let mut size = (text.len() * 2) as u32;
    // SAFETY: 缓冲与长度（字节）对应，字符串是编译期的 UTF-16 常量。
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!(r"Software\Microsoft\Windows NT\CurrentVersion"),
            w!("CurrentBuildNumber"),
            RRF_RT_REG_SZ,
            None,
            Some(text.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if status.is_err() {
        return None;
    }
    String::from_utf16_lossy(&text)
        .trim_end_matches('\0')
        .parse()
        .ok()
}

#[cfg(target_os = "windows")]
fn read_palette() -> Option<[u8; 32]> {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_BINARY, RegGetValueW};
    use windows::core::w;

    let mut palette = [0u8; 32];
    let mut size = palette.len() as u32;
    // SAFETY: 缓冲与长度对应，字符串是编译期的 UTF-16 常量。
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent"),
            w!("AccentPalette"),
            RRF_RT_REG_BINARY,
            None,
            Some(palette.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    (status.is_ok() && size as usize == palette.len()).then_some(palette)
}

#[cfg(test)]
mod tests {
    use super::Accent;
    use crate::color::Color;

    #[test]
    fn picks_the_shades_the_system_uses() {
        let palette = [
            0x99, 0xEB, 0xFF, 0, 0x4C, 0xC2, 0xFF, 0, 0x00, 0x91, 0xF8, 0, 0x00, 0x78, 0xD4, 0,
            0x00, 0x67, 0xC0, 0, 0x00, 0x3E, 0x92, 0, 0x00, 0x1A, 0x68, 0, 0xF7, 0x63, 0x0C, 0,
        ];
        assert_eq!(Accent::from_palette(&palette), Some(Accent::FLUENT_BLUE));
        assert_eq!(Accent::FLUENT_BLUE.on_dark, Color::rgb(0x4C, 0xC2, 0xFF));
    }

    #[test]
    fn a_short_palette_is_rejected() {
        assert_eq!(Accent::from_palette(&[0; 8]), None);
    }
}
