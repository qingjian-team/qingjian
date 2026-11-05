//! `.qjz`：分发用的压缩 `.qj`（词库仓库的 Release 附件、设置页导入与下载）。
//!
//! 只用来传输：装进用户目录时先解成 `.qj`，加载照旧 mmap `.qj`。直接读压缩流就得把整份解到堆上，
//! 白占几十 MB 常驻内存，mmap 按需换页的好处也没了。
//! 压缩算法按文件开头的魔数认，现在只有 gzip；以后换 zstd / xz 时扩展名不变，旧文件照样能装。

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use crate::{Container, FormatError, Kind, Metadata};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

/// 解压后最多这么大：随包最大的 `.qj`（语言模型）不到 50 MB，留足余量，挡住压缩炸弹。
const MAX_UNPACKED: u64 = 1 << 30;

/// 文件开头是不是认得的压缩格式。
pub fn is_qjz(path: &Path) -> bool {
    let mut magic = [0u8; GZIP_MAGIC.len()];
    File::open(path)
        .and_then(|mut f| f.read_exact(&mut magic))
        .is_ok()
        && magic == GZIP_MAGIC
}

/// 把 `.qj` 压成 `.qjz`；源文件不是 `.qj` 就拒绝。
pub fn compress(source: &Path, target: &Path) -> Result<(), FormatError> {
    if !Container::is_qj(source) {
        return Err(FormatError::BadMagic);
    }
    let mut input = BufReader::new(File::open(source)?);
    let mut encoder = GzEncoder::new(BufWriter::new(File::create(target)?), Compression::best());
    std::io::copy(&mut input, &mut encoder)?;
    encoder.finish()?.flush()?;
    Ok(())
}

/// 把 `.qjz` 解成 `target`：先写到旁边的临时文件（扩展名 `.qj.part`，扫 `*.qj` 的不会看见），
/// 按 `expected` 整份校验一遍，通过了才改名换上；种类不对、中途失败都不动原来的 `target`。返回解出的元数据。
pub fn unpack(source: &Path, target: &Path, expected: Kind) -> Result<Metadata, FormatError> {
    let mut magic = [0u8; GZIP_MAGIC.len()];
    File::open(source)?.read_exact(&mut magic)?;
    if magic != GZIP_MAGIC {
        return Err(FormatError::UnknownCompression);
    }
    let part = target.with_extension("qj.part");
    let result = unpack_to(source, &part, expected).and_then(|checked| {
        std::fs::rename(&part, target)?;
        Ok(checked)
    });
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result
}

fn unpack_to(source: &Path, part: &Path, expected: Kind) -> Result<Metadata, FormatError> {
    let decoder = GzDecoder::new(BufReader::new(File::open(source)?));
    let mut output = BufWriter::new(File::create(part)?);
    let written = std::io::copy(&mut decoder.take(MAX_UNPACKED + 1), &mut output)?;
    output.flush()?;
    drop(output);
    if written > MAX_UNPACKED {
        return Err(FormatError::TooLarge);
    }
    // 校验完就放掉映射：Windows 上映射着的文件不能改名
    Ok(Container::open(part, expected)?.metadata().clone())
}

#[cfg(test)]
mod tests;
