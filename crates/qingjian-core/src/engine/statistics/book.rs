/// 给累计字数一个直观参照的书。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Book {
    /// 书名，不带书名号。
    pub title: &'static str,

    /// 约多少汉字（不含标点，取整到万或千）。
    pub hanzi: u64,
}

/// 参照书目，按字数从薄到厚。字数是通行版本的约数，只求量级对。
pub const BOOKS: &[Book] = &[
    Book {
        title: "道德经",
        hanzi: 5_000,
    },
    Book {
        title: "论语",
        hanzi: 16_000,
    },
    Book {
        title: "小王子",
        hanzi: 25_000,
    },
    Book {
        title: "边城",
        hanzi: 60_000,
    },
    Book {
        title: "活着",
        hanzi: 120_000,
    },
    Book {
        title: "围城",
        hanzi: 250_000,
    },
    Book {
        title: "三国演义",
        hanzi: 640_000,
    },
    Book {
        title: "红楼梦",
        hanzi: 730_000,
    },
    Book {
        title: "西游记",
        hanzi: 820_000,
    },
    Book {
        title: "平凡的世界",
        hanzi: 1_000_000,
    },
];

/// 挑一本书对照 `hanzi` 个字：不超过它的书里最厚的一本，一本都不到就拿最薄的。返回那本书与倍数。
pub fn book_scale(hanzi: u64) -> (&'static Book, f64) {
    let book = BOOKS
        .iter()
        .rev()
        .find(|book| book.hanzi <= hanzi)
        .unwrap_or(&BOOKS[0]);
    (book, hanzi as f64 / book.hanzi as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn books_are_sorted_from_thin_to_thick() {
        assert!(BOOKS.windows(2).all(|pair| pair[0].hanzi < pair[1].hanzi));
    }

    #[test]
    fn picks_the_thickest_book_not_exceeding_the_count() {
        let (book, ratio) = book_scale(0);
        assert_eq!(book.title, "道德经");
        assert_eq!(ratio, 0.0);
        let (book, ratio) = book_scale(240_000);
        assert_eq!(book.title, "活着");
        assert_eq!(ratio, 2.0);
        let (book, _) = book_scale(5_000_000);
        assert_eq!(book.title, "平凡的世界");
    }
}
