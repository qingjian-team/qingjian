use rusqlite::Connection;

use super::ClipStore;
use crate::capture::{Capture, ClipFormat, FORMAT_HTML, FORMAT_THUMBNAIL};
use crate::kind::{ClipFilter, ClipKind};

fn store(limit: usize) -> ClipStore {
    ClipStore::with_connection(Connection::open_in_memory().unwrap(), limit).unwrap()
}

fn plain(text: &str) -> Capture {
    Capture::text(text.to_owned(), String::new(), Vec::new())
}

fn texts(store: &ClipStore, query: &str) -> Vec<String> {
    store
        .search(query, ClipFilter::All, 100)
        .unwrap()
        .into_iter()
        .map(|item| item.text)
        .collect()
}

#[test]
fn newest_first_and_repeats_move_to_front() {
    let mut store = store(10);
    store.add(&plain("a"), 1).unwrap();
    store.add(&plain("b"), 2).unwrap();
    store.add(&plain("a"), 3).unwrap();
    assert_eq!(texts(&store, ""), ["a", "b"]);
}

#[test]
fn blank_text_is_ignored_and_formatting_is_kept() {
    let mut store = store(10);
    store.add(&plain(" \n\t"), 1).unwrap();
    store.add(&plain("  fn main() {\n\t}\n"), 2).unwrap();
    assert_eq!(texts(&store, ""), ["  fn main() {\n\t}\n"]);
}

#[test]
fn search_treats_wildcards_literally() {
    let mut store = store(10);
    store.add(&plain("100% 完成"), 1).unwrap();
    store.add(&plain("100 完成"), 2).unwrap();
    store.add(&plain("Snake_Case"), 3).unwrap();
    assert_eq!(texts(&store, "0%"), ["100% 完成"]);
    assert_eq!(texts(&store, "e_c"), ["Snake_Case"]);
    assert_eq!(texts(&store, "完成").len(), 2);
}

#[test]
fn over_the_limit_drops_the_oldest_with_its_formats() {
    let mut store = store(2);
    store
        .add(&Capture::image(vec![1], vec![2], String::new()), 0)
        .unwrap();
    let image = store.search("", ClipFilter::Image, 1).unwrap().remove(0);
    store.add(&plain("b"), 1).unwrap();
    store.add(&plain("c"), 2).unwrap();
    assert_eq!(texts(&store, ""), ["c", "b"]);
    assert!(store.formats(&image.id).unwrap().is_empty());
}

#[test]
fn delete_erases_everything_but_keeps_a_tombstone() {
    let mut store = store(10);
    let html = vec![ClipFormat::new(FORMAT_HTML, b"<b>secret</b>".to_vec())];
    let capture = Capture::text("secret".to_owned(), "app.exe".to_owned(), html);
    store.add(&capture, 1).unwrap();
    let id = store.search("", ClipFilter::All, 1).unwrap().remove(0).id;
    store.delete(&id, 2).unwrap();
    assert!(texts(&store, "").is_empty());
    assert!(store.formats(&id).unwrap().is_empty());
    let (text, source, deleted): (String, String, i64) = store
        .connection
        .query_row(
            "SELECT text, source, deleted FROM items WHERE id = ?1",
            [&id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!((text.as_str(), source.as_str(), deleted), ("", "", 1));
    store.add(&capture, 3).unwrap();
    assert_ne!(store.search("", ClipFilter::All, 1).unwrap()[0].id, id);
}

#[test]
fn ids_stay_unique_across_inserts() {
    let mut store = store(10);
    store.add(&plain("a"), 1).unwrap();
    store.add(&plain("b"), 2).unwrap();
    let items = store.search("", ClipFilter::All, 10).unwrap();
    assert_ne!(items[0].id, items[1].id);
    assert!(items[0].id.starts_with(&store.device_id));
}

#[test]
fn kinds_are_classified_and_filtered() {
    let mut store = store(10);
    store.add(&plain("plain"), 1).unwrap();
    let html = vec![ClipFormat::new(FORMAT_HTML, b"<b>rich</b>".to_vec())];
    store
        .add(&Capture::text("rich".to_owned(), String::new(), html), 2)
        .unwrap();
    store.add(&plain("fn main() {\n    run();\n}"), 3).unwrap();
    store
        .add(&Capture::image(vec![1], vec![2], String::new()), 4)
        .unwrap();
    let paths = ["C:\\a.txt".to_owned()];
    store
        .add(&Capture::files(&paths, vec![9], String::new(), None), 5)
        .unwrap();
    let kinds = |filter| -> Vec<ClipKind> {
        store
            .search("", filter, 10)
            .unwrap()
            .into_iter()
            .map(|item| item.kind)
            .collect()
    };
    assert_eq!(kinds(ClipFilter::All).len(), 5);
    assert_eq!(kinds(ClipFilter::Text), [ClipKind::Rich, ClipKind::Text]);
    assert_eq!(kinds(ClipFilter::Code), [ClipKind::Code]);
    assert_eq!(kinds(ClipFilter::Image), [ClipKind::Image]);
    assert_eq!(kinds(ClipFilter::Files), [ClipKind::Files]);
}

#[test]
fn same_image_with_another_thumbnail_is_the_same_item() {
    let mut store = store(10);
    store
        .add(&Capture::image(vec![1], vec![2], String::new()), 1)
        .unwrap();
    store
        .add(&Capture::image(vec![1], vec![3], String::new()), 2)
        .unwrap();
    let items = store.search("", ClipFilter::All, 10).unwrap();
    assert_eq!(items.len(), 1);
    let thumbnail = store.format(&items[0].id, FORMAT_THUMBNAIL).unwrap();
    assert_eq!(thumbnail, Some(vec![2]));
}

#[test]
fn editing_replaces_the_text_and_drops_stale_formats() {
    let mut store = store(10);
    let html = vec![ClipFormat::new(FORMAT_HTML, b"<b>old</b>".to_vec())];
    store
        .add(&Capture::text("old".to_owned(), String::new(), html), 1)
        .unwrap();
    let id = store.search("", ClipFilter::All, 1).unwrap().remove(0).id;
    store.edit_text(&id, "fn new() {\n    go();\n}", 2).unwrap();
    let item = store.search("", ClipFilter::All, 1).unwrap().remove(0);
    assert_eq!(item.id, id);
    assert_eq!(item.kind, ClipKind::Code);
    assert_eq!(item.updated_at, 2);
    assert!(store.formats(&id).unwrap().is_empty());
}

#[test]
fn touching_moves_an_item_to_the_front() {
    let mut store = store(10);
    store.add(&plain("a"), 1).unwrap();
    store.add(&plain("b"), 2).unwrap();
    let last = store.search("", ClipFilter::All, 10).unwrap().remove(1);
    store.touch(&last.id, 3).unwrap();
    assert_eq!(texts(&store, ""), ["a", "b"]);
}

#[test]
fn clear_removes_everything_but_keeps_tombstones() {
    let path = std::env::temp_dir().join(format!("qj-clip-clear-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut store = ClipStore::open(&path, 10).unwrap();
    let capture = Capture::text(
        "QJ-very-secret".to_owned(),
        "app.exe".to_owned(),
        Vec::new(),
    );
    store.add(&capture, 1).unwrap();
    assert_eq!(store.search("", ClipFilter::All, 10).unwrap().len(), 1);
    store.clear(2).unwrap();
    assert!(store.search("", ClipFilter::All, 10).unwrap().is_empty());
    let tombstones: i64 = store
        .connection
        .query_row(
            "SELECT COUNT(*) FROM items WHERE deleted = 1 AND fingerprint = ''",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tombstones, 1);
    // 同样的内容再复制是新的一条
    store.add(&capture, 3).unwrap();
    store.clear(4).unwrap();
    drop(store);
    let bytes = std::fs::read(&path).unwrap();
    assert!(!bytes.windows(14).any(|window| window == b"QJ-very-secret"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn too_long_text_is_skipped_not_truncated() {
    let mut store = ClipStore::with_connection(Connection::open_in_memory().unwrap(), 10).unwrap();
    let longest = "好".repeat(crate::capture::MAX_TEXT_CHARS);
    store
        .add(
            &Capture::text(longest.clone(), String::new(), Vec::new()),
            1,
        )
        .unwrap();
    store
        .add(
            &Capture::text(format!("{longest}好"), String::new(), Vec::new()),
            2,
        )
        .unwrap();
    let items = store.search("", ClipFilter::All, 10).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].text.chars().count(),
        crate::capture::MAX_TEXT_CHARS
    );
}

#[test]
fn oldest_items_go_first_when_over_the_size_budget() {
    let mut store = ClipStore::with_connection(Connection::open_in_memory().unwrap(), 100).unwrap();
    store.byte_limit = 100;
    let image = |byte: u8| Capture::image(vec![byte; 60], vec![byte; 1], String::new());
    store.add(&image(1), 1).unwrap();
    store.add(&image(2), 2).unwrap();
    store.add(&image(3), 3).unwrap();
    let items = store.search("", ClipFilter::All, 10).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(
        store
            .format(&items[0].id, crate::capture::FORMAT_PNG)
            .unwrap()
            .unwrap()[0],
        3
    );
    let left: i64 = store
        .connection
        .query_row("SELECT COUNT(*) FROM formats", [], |row| row.get(0))
        .unwrap();
    assert_eq!(left, 2);
    // 单独一条超了也留着：刚复制的不能一记就丢
    store
        .add(&Capture::image(vec![9; 500], vec![9; 1], String::new()), 4)
        .unwrap();
    assert_eq!(store.search("", ClipFilter::All, 10).unwrap().len(), 1);
}

#[test]
fn lowering_the_limit_drops_the_oldest_at_once() {
    let mut store = ClipStore::with_connection(Connection::open_in_memory().unwrap(), 10).unwrap();
    for (index, text) in ["one", "two", "three"].into_iter().enumerate() {
        store.add(&plain(text), index as i64).unwrap();
    }
    store.set_limit(2, 9).unwrap();
    let items = store.search("", ClipFilter::All, 10).unwrap();
    let texts: Vec<_> = items.iter().map(|item| item.text.as_str()).collect();
    assert_eq!(texts, ["three", "two"]);
    store.set_limit(10, 10).unwrap();
    assert_eq!(store.search("", ClipFilter::All, 10).unwrap().len(), 2);
}

#[test]
fn image_info_reads_the_png_header() {
    let mut store = ClipStore::with_connection(Connection::open_in_memory().unwrap(), 10).unwrap();
    let mut png = vec![0; 16];
    png.extend_from_slice(&1920_u32.to_be_bytes());
    png.extend_from_slice(&1080_u32.to_be_bytes());
    png.extend_from_slice(&[0; 100]);
    let size = png.len() as u64;
    store
        .add(&Capture::image(png, vec![1], String::new()), 1)
        .unwrap();
    store
        .add(&Capture::image(vec![1, 2], vec![2], String::new()), 2)
        .unwrap();
    store.add(&plain("words"), 3).unwrap();
    let items = store.search("", ClipFilter::All, 10).unwrap();
    assert_eq!(
        store.image_info(&items[2].id).unwrap(),
        Some((1920, 1080, size))
    );
    // 存坏了的、不是图片的都没有
    assert_eq!(store.image_info(&items[1].id).unwrap(), None);
    assert_eq!(store.image_info(&items[0].id).unwrap(), None);
}
