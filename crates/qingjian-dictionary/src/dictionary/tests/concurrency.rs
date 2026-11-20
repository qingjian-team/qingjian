//! 编译期 Send/Sync 断言与多线程只读查询稳定性。

use super::*;

use std::sync::Arc;

#[test]
fn dictionary_is_send_and_sync() {
    assert_send_sync::<Dictionary>();
}

#[test]
fn concurrent_reads_return_stable_results() {
    let dictionary = Arc::new(Dictionary::parse(GOLDEN).unwrap());
    let baseline_a = owned_matches(&dictionary.lookup(&["kai", "fa"], false));
    let baseline_b = owned_matches(&dictionary.lookup(&["k", "f"], true));

    let mut handles = Vec::new();
    for _ in 0..8 {
        let dictionary = Arc::clone(&dictionary);
        handles.push(std::thread::spawn(move || {
            let mut out = Vec::new();
            for _ in 0..50 {
                out.push(owned_matches(&dictionary.lookup(&["kai", "fa"], false)));
                out.push(owned_matches(&dictionary.lookup(&["k", "f"], true)));
            }
            out
        }));
    }
    for handle in handles {
        let results = handle.join().unwrap();
        for chunk in results.chunks(2) {
            assert_eq!(chunk[0], baseline_a);
            assert_eq!(chunk[1], baseline_b);
        }
    }
}
