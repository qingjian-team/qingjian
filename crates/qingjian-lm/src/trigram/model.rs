//! 打开 `.ngram` 文件、校验各数组的边界，并按 ARPA 的回退规则查 log10 P。

use std::fs::File;
use std::path::Path;

use memmap2::Mmap;

use crate::error::LmError;

/// 文件头的魔数。
const MAGIC: &[u8; 8] = b"ZYNGRAM1";

/// 量化值 × 这个数是 log10 P（或回退权重）。
const SCALE: f64 = 1.0 / 2048.0;

/// 映射着的三元模型。各数组只记在文件里的字节起点，按需读。
pub struct TrigramModel {
    map: Mmap,

    /// 一元、二元、三元的条数。
    n1: usize,
    n2: usize,
    n3: usize,

    word_off: usize,
    words: usize,
    p1: usize,
    b1: usize,
    bi_start: usize,
    bi_w: usize,
    bi_p: usize,
    bi_b: usize,
    tri_start: usize,
    tri_w: usize,
    tri_p: usize,

    /// 不在词表里的字按它算：`<unk>`，没有就是 0 号词。
    unknown: u32,
}

impl TrigramModel {
    /// 映射并校验：魔数不对、数组越界或分段对不上都报 [`LmError::Corrupt`]。
    pub fn open(path: &Path) -> Result<Self, LmError> {
        let file = File::open(path)?;
        // SAFETY: 只读映射，文件在使用期间不被本进程改写；外部改写同 `.qj` 一样不在防范范围内。
        let map = unsafe { Mmap::map(&file)? };
        if map.len() < 24 || &map[..8] != MAGIC {
            return Err(LmError::Corrupt("not a .ngram file"));
        }
        let header = |i: usize| read_u32(&map, 8 + 4 * i) as usize;
        let (n1, n2, n3) = (header(0), header(1), header(2));
        if n1 == 0 {
            return Err(LmError::Corrupt("empty n-gram vocabulary"));
        }
        let mut cursor = Cursor {
            pos: 24,
            len: map.len(),
        };
        let word_off = cursor.take((n1 + 1) * 4)?;
        let words_len = read_u32(&map, word_off + 4 * n1) as usize;
        let words = cursor.take(words_len)?;
        let p1 = cursor.take(n1 * 2)?;
        let b1 = cursor.take(n1 * 2)?;
        let bi_start = cursor.take((n1 + 1) * 4)?;
        let bi_w = cursor.take(n2 * 4)?;
        let bi_p = cursor.take(n2 * 2)?;
        let bi_b = cursor.take(n2 * 2)?;
        let tri_start = cursor.take((n2 + 1) * 4)?;
        let tri_w = cursor.take(n3 * 4)?;
        let tri_p = cursor.take(n3 * 2)?;
        if read_u32(&map, bi_start + 4 * n1) as usize != n2
            || read_u32(&map, tri_start + 4 * n2) as usize != n3
        {
            return Err(LmError::Corrupt("n-gram sections are inconsistent"));
        }
        let mut model = Self {
            map,
            n1,
            n2,
            n3,
            word_off,
            words,
            p1,
            b1,
            bi_start,
            bi_w,
            bi_p,
            bi_b,
            tri_start,
            tri_w,
            tri_p,
            unknown: 0,
        };
        model.unknown = model.word("<unk>").unwrap_or(0);
        tracing::debug!(unigrams = n1, bigrams = n2, trigrams = n3, "三元模型已映射");
        Ok(model)
    }

    pub fn unigram_count(&self) -> usize {
        self.n1
    }

    pub fn bigram_count(&self) -> usize {
        self.n2
    }

    pub fn trigram_count(&self) -> usize {
        self.n3
    }

    pub fn bytes(&self) -> usize {
        self.map.len()
    }

    /// 词表里的每个词与它的一元 log10 P，按字节序。`<unk>`、`<s>` 这类记号也在里面，调用方自己滤。
    pub fn vocabulary(&self) -> impl Iterator<Item = (&str, f64)> + '_ {
        (0..self.n1).filter_map(|i| {
            let text = std::str::from_utf8(self.word_bytes(i)).ok()?;
            Some((text, self.unigram(i as u32)))
        })
    }

    pub(super) fn unknown(&self) -> u32 {
        self.unknown
    }

    /// 词的编号：词表按字节序排好，二分查。
    pub(super) fn word(&self, text: &str) -> Option<u32> {
        let target = text.as_bytes();
        let (mut lo, mut hi) = (0, self.n1);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match self.word_bytes(mid).cmp(target) {
                std::cmp::Ordering::Equal => return Some(mid as u32),
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
            }
        }
        None
    }

    /// log10 P(w | u v)，`u` / `v` 为 `None` 是没有那么长的前文。三元没有就退到二元加回退权重，二元没有再退到一元。
    pub(super) fn log10(&self, u: Option<u32>, v: Option<u32>, w: u32) -> f64 {
        let Some(v) = v else {
            return self.unigram(w);
        };
        let Some(u) = u else {
            return self.bigram_log10(v, w);
        };
        // 前两个词本身不成二元时，三元的历史不存在，按 ARPA 不加回退权重直接退
        let Some(history) = self.bigram(u, v) else {
            return self.bigram_log10(v, w);
        };
        let (start, end) = self.segment(self.tri_start, history);
        match self.find(self.tri_w, start, end, w) {
            Some(k) => self.value(self.tri_p, k),
            None => self.value(self.bi_b, history) + self.bigram_log10(v, w),
        }
    }

    fn bigram_log10(&self, v: u32, w: u32) -> f64 {
        match self.bigram(v, w) {
            Some(j) => self.value(self.bi_p, j),
            None => self.value(self.b1, v as usize) + self.unigram(w),
        }
    }

    fn unigram(&self, w: u32) -> f64 {
        self.value(self.p1, w as usize)
    }

    /// (v, w) 这条二元的下标。
    fn bigram(&self, v: u32, w: u32) -> Option<usize> {
        let (start, end) = self.segment(self.bi_start, v as usize);
        self.find(self.bi_w, start, end, w)
    }

    /// 分段表 `starts` 里第 `i` 段的 [起, 止)。
    fn segment(&self, starts: usize, i: usize) -> (usize, usize) {
        (
            read_u32(&self.map, starts + 4 * i) as usize,
            read_u32(&self.map, starts + 4 * (i + 1)) as usize,
        )
    }

    /// 在按编号升序的 `ids[start..end]` 里找 `w`，返回它的下标。
    fn find(&self, ids: usize, start: usize, end: usize, w: u32) -> Option<usize> {
        let (mut lo, mut hi) = (start, end);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match read_u32(&self.map, ids + 4 * mid).cmp(&w) {
                std::cmp::Ordering::Equal => return Some(mid),
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
            }
        }
        None
    }

    fn value(&self, array: usize, i: usize) -> f64 {
        f64::from(read_i16(&self.map, array + 2 * i)) * SCALE
    }

    fn word_bytes(&self, i: usize) -> &[u8] {
        let start = read_u32(&self.map, self.word_off + 4 * i) as usize;
        let end = read_u32(&self.map, self.word_off + 4 * (i + 1)) as usize;
        self.map
            .get(self.words + start..self.words + end)
            .unwrap_or_default()
    }
}

/// 顺序切出各数组：每个数组先对齐到 8 字节，越界就报错。
struct Cursor {
    pos: usize,
    len: usize,
}

impl Cursor {
    fn take(&mut self, bytes: usize) -> Result<usize, LmError> {
        let start = self.pos.div_ceil(8) * 8;
        let end = start
            .checked_add(bytes)
            .filter(|&end| end <= self.len)
            .ok_or(LmError::Corrupt("n-gram file is shorter than its counts"))?;
        self.pos = end;
        Ok(start)
    }
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    bytes
        .get(at..at + 4)
        .map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn read_i16(bytes: &[u8], at: usize) -> i16 {
    bytes
        .get(at..at + 2)
        .map_or(0, |b| i16::from_le_bytes([b[0], b[1]]))
}
