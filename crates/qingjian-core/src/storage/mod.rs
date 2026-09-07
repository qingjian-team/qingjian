//! 学习数据、配置这类小文件的落盘方式：先写同目录的临时文件，落盘后再改名覆盖。
//!
//! 输入法进程随时可能被 launchd 杀掉或崩溃；直接 `File::create` 覆盖写，中途断掉就留下半个文件，
//! 用户攒的词频 / n-gram 整个报废。改名在同一文件系统上是原子的，所以任何时刻磁盘上要么是旧文件、要么是新文件。

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

/// 临时文件的后缀：与目标同目录（改名不能跨文件系统），带进程号避免两个进程互相覆盖。
fn temporary_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.tmp-{}", std::process::id()))
}

/// 原子地写 `path`：`write` 往缓冲写入端里写内容，写完 flush + fsync，再改名覆盖目标。
/// 中途任何一步失败都不碰目标文件，临时文件也清掉。
pub fn write_atomic(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> io::Result<()> {
    write_atomic_with(path, false, write)
}

/// 同 [`write_atomic`]，但文件只有本用户可读写（0600）：放密钥的 `.env` 用。
pub fn write_atomic_private(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> io::Result<()> {
    write_atomic_with(path, true, write)
}

fn write_atomic_with(
    path: &Path,
    private: bool,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> io::Result<()> {
    let temporary = temporary_path(path);
    let result = (|| {
        let file = create(&temporary, private)?;
        let mut writer = BufWriter::new(file);
        write(&mut writer)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// 建临时文件；`private` 时权限 0600（非 Unix 平台没有这个概念，照常建）。
fn create(path: &Path, private: bool) -> io::Result<File> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    #[cfg(not(unix))]
    let _ = private;
    options.open(path)
}

/// 把 `text` 原子地写进 `path`。
pub fn write_atomic_str(path: &Path, text: &str) -> io::Result<()> {
    write_atomic(path, |writer| writer.write_all(text.as_bytes()))
}

/// 读一个可能被写坏的文本文件：不存在返回 `None`；不是合法 UTF-8 的部分按替换字符读进来，让按行解析去跳过坏行，
/// 而不是整个文件读不了。其余 io 错误照常返回。
pub fn read_text_lossy(path: &Path) -> io::Result<Option<String>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每个测试一个目录：测试并行跑，共用目录时一个测试的临时文件会被另一个测试的目录扫描撞见
    fn scratch(test: &str, name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("qingjian-storage-{}-{test}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn atomic_write_replaces_the_file_and_leaves_no_temporary() {
        let path = scratch("atomic", "atomic.tsv");
        write_atomic_str(&path, "old\n").unwrap();
        write_atomic_str(&path, "new\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
        let leftovers: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(&format!("tmp-{}", std::process::id()))
            })
            .collect();
        assert!(leftovers.is_empty(), "临时文件没清：{leftovers:?}");
    }

    #[test]
    fn failed_write_keeps_the_old_file() {
        let path = scratch("kept", "kept.tsv");
        write_atomic_str(&path, "old\n").unwrap();
        let result = write_atomic(&path, |writer| {
            writer.write_all(b"half")?;
            Err(io::Error::other("boom"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "old\n");
        assert!(!temporary_path(&path).exists());
    }

    #[cfg(unix)]
    #[test]
    fn private_write_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let path = scratch("private", "private.env");
        write_atomic_private(&path, |writer| writer.write_all(b"KEY=1\n")).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn lossy_read_tolerates_broken_utf8_and_missing_files() {
        let path = scratch("lossy", "broken.tsv");
        fs::write(&path, b"\xe5\xa5\xbd\t1\n\xe5\xa5\t2\n").unwrap();
        let text = read_text_lossy(&path).unwrap().unwrap();
        assert!(text.starts_with("好\t1\n"));
        assert!(text.contains('\u{FFFD}'));
        assert!(
            read_text_lossy(&scratch("lossy", "missing.tsv"))
                .unwrap()
                .is_none()
        );
    }
}
