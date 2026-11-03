//! 一次复制读到的内容：文字加系统剪贴板的原始格式，粘贴时原样写回去，格式才不丢。

mod format;

use qingjian_code::looks_like_code;

use crate::kind::ClipKind;

pub use format::ClipFormat;

/// 一条文字最多多少字符，再长的整条不记：截断了存，粘贴出来就是残缺的。
pub const MAX_TEXT_CHARS: usize = 1_000_000;

/// HTML、RTF 一种格式最多多大（字节），再大的丢掉格式、只留文字。
pub const MAX_FORMAT_BYTES: usize = 8 * 1024 * 1024;

/// 文件列表最多多大（字节，约几万个路径）。文件本身不读，复制多大的文件都只记路径。
pub const MAX_FILE_LIST_BYTES: usize = 4 * 1024 * 1024;

/// 图片编码后最多多大（字节）。
pub const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;

/// [`ClipFormat::name`] 的取值。
pub const FORMAT_HTML: &str = "html";

pub const FORMAT_RTF: &str = "rtf";

/// 文件列表，系统剪贴板的原始字节。
pub const FORMAT_FILES: &str = "files";

pub const FORMAT_PNG: &str = "png";

/// 卡片上显示的缩略图（PNG）；不算内容，不参与去重。
pub const FORMAT_THUMBNAIL: &str = "thumbnail";

/// 复制时这些文件一共多大（字节，十进制文字）；同样只给卡片看。
pub const FORMAT_FILE_SIZE: &str = "file_size";

/// 给人看的大小：`512 B`、`1.5 KB`、`3.2 GB`，1024 进制，同 Windows 资源管理器。
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["KB", "MB", "GB", "TB", "PB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut size = bytes as f64 / 1024.0;
    let mut unit = 0;
    while size >= 1024.0 && unit + 1 < UNITS.len() {
        size /= 1024.0;
        unit += 1;
    }
    if size < 100.0 {
        format!("{size:.1} {}", UNITS[unit])
    } else {
        format!("{size:.0} {}", UNITS[unit])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    pub kind: ClipKind,

    /// 文字；文件是一行一个路径，图片为空。
    pub text: String,

    /// 来源程序的路径，不知道时为空。
    pub source: String,

    pub formats: Vec<ClipFormat>,
}

impl Capture {
    /// 文字：像代码的算代码（编辑器复制代码也带 HTML，但用户要的是代码），带格式的算富文本。
    pub fn text(text: String, source: String, formats: Vec<ClipFormat>) -> Self {
        let kind = text_kind(&text, !formats.is_empty());
        Self {
            kind,
            text,
            source,
            formats,
        }
    }

    pub fn image(png: Vec<u8>, thumbnail: Vec<u8>, source: String) -> Self {
        Self {
            kind: ClipKind::Image,
            text: String::new(),
            source,
            formats: vec![
                ClipFormat::new(FORMAT_PNG, png),
                ClipFormat::new(FORMAT_THUMBNAIL, thumbnail),
            ],
        }
    }

    /// 既是文件又是位图（有的程序复制图片时给一个临时文件加几种图片格式）：按图片记，临时文件被清理了照样能粘贴；
    /// 文件列表也留着，粘贴时文件还在就一起写回。
    pub fn image_with_files(
        png: Vec<u8>,
        thumbnail: Vec<u8>,
        files: Vec<u8>,
        source: String,
    ) -> Self {
        let mut capture = Self::image(png, thumbnail, source);
        capture.formats.push(ClipFormat::new(FORMAT_FILES, files));
        capture
    }

    /// `size` 是这些文件的总大小，不知道（有文件夹、在网络盘上）时为 `None`。
    pub fn files(paths: &[String], raw: Vec<u8>, source: String, size: Option<u64>) -> Self {
        let mut formats = vec![ClipFormat::new(FORMAT_FILES, raw)];
        if let Some(size) = size {
            formats.push(ClipFormat::new(
                FORMAT_FILE_SIZE,
                size.to_string().into_bytes(),
            ));
        }
        Self {
            kind: ClipKind::Files,
            text: paths.join("\n"),
            source,
            formats,
        }
    }

    /// 没有任何内容（全是空白的文字）。
    pub(crate) fn is_empty(&self) -> bool {
        self.text.trim().is_empty() && self.formats.is_empty()
    }

    /// 内容相同的指纹（FNV-1a）：同样的内容再复制时挪到最前。
    pub(crate) fn fingerprint(&self) -> String {
        let mut hash = fingerprint_bytes(FNV_OFFSET, self.text.as_bytes());
        for format in &self.formats {
            // 图片不看文件列表：同一张图每次复制的临时路径都不同
            let path_of_image = self.kind == ClipKind::Image && format.name == FORMAT_FILES;
            if format.name != FORMAT_THUMBNAIL && format.name != FORMAT_FILE_SIZE && !path_of_image
            {
                hash = fingerprint_bytes(hash, format.name.as_bytes());
                hash = fingerprint_bytes(hash, &format.data);
            }
        }
        format!("{hash:016x}-{}", self.text.len())
    }
}

pub(crate) fn text_kind(text: &str, has_formats: bool) -> ClipKind {
    if looks_like_code(text) {
        ClipKind::Code
    } else if has_formats {
        ClipKind::Rich
    } else {
        ClipKind::Text
    }
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

fn fingerprint_bytes(seed: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(seed, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::{Capture, format_size};

    #[test]
    fn sizes_read_naturally() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(150 * 1024), "150 KB");
        assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3.0 GB");
        assert_eq!(format_size(5 * 1024_u64.pow(4)), "5.0 TB");
    }

    #[test]
    fn the_same_image_under_another_temp_path_is_the_same_copy() {
        let copy =
            |path: u8| Capture::image_with_files(vec![1], vec![2], vec![path], String::new());
        assert_eq!(copy(1).fingerprint(), copy(2).fingerprint());
        assert_eq!(
            copy(1).fingerprint(),
            Capture::image(vec![1], vec![2], String::new()).fingerprint()
        );
    }

    #[test]
    fn a_file_that_grew_is_still_the_same_copy() {
        let paths = ["C:\\a.txt".to_owned()];
        let small = Capture::files(&paths, vec![1], String::new(), Some(1));
        let large = Capture::files(&paths, vec![1], String::new(), Some(2));
        assert_eq!(small.fingerprint(), large.fingerprint());
    }
}
