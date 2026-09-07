use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use super::FILE_PREFIX;

/// 按天分文件的日志写入端：日期变了换文件，文件在磁盘上被删了就重开（`nlink == 0` 说明当前描述符指向的
/// inode 已经从目录里摘掉）。每次写之前 fstat 一次，代价可以忽略，且写入本来就在 `non_blocking` 的后台线程。
pub struct LogFile {
    /// 日志目录。
    dir: PathBuf,

    /// 当前打开的文件及其日期；打不开时为 `None`，下次写再试。
    current: Option<(jiff::civil::Date, File)>,
}

impl LogFile {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir, current: None }
    }

    /// 某一天的日志文件路径。
    pub fn path_for(dir: &Path, date: jiff::civil::Date) -> PathBuf {
        dir.join(format!("{FILE_PREFIX}.{date}"))
    }

    /// 拿到今天的文件：日期变了、还没打开、或者文件已被删掉时重新打开（追加模式）。
    fn file(&mut self) -> io::Result<&mut File> {
        let today = jiff::Zoned::now().date();
        let stale = match &self.current {
            Some((date, file)) => *date != today || file.metadata().is_ok_and(|m| m.nlink() == 0),
            None => true,
        };
        if stale {
            std::fs::create_dir_all(&self.dir)?;
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(Self::path_for(&self.dir, today))?;
            self.current = Some((today, file));
        }
        Ok(&mut self.current.as_mut().expect("just opened").1)
    }
}

impl Write for LogFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file()?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.current {
            Some((_, file)) => file.flush(),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reopens_after_the_file_is_deleted() {
        let dir = std::env::temp_dir().join("qingjian-log-file-test");
        let _ = std::fs::remove_dir_all(&dir);
        let mut log = LogFile::new(dir.clone());
        log.write_all(b"one\n").unwrap();
        let path = LogFile::path_for(&dir, jiff::Zoned::now().date());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "one\n");
        std::fs::remove_file(&path).unwrap();
        log.write_all(b"two\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "two\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
