//! 用量缺失、缓存计数、持久化与并发更新的回归测试；不访问外部 API。

use std::path::PathBuf;

use serde_json::json;

use super::ApiUsageStats;

fn scratch() -> PathBuf {
    std::env::temp_dir()
        .join(format!("qingjian-api-usage-{}", uuid::Uuid::new_v4()))
        .join("api-usage.json")
}

#[test]
fn reported_usage_survives_reload_and_unknown_usage_is_separate() {
    let path = scratch();
    assert_eq!(ApiUsageStats::load(&path).unwrap().requests, 0);
    ApiUsageStats::record_request(&path);
    assert_eq!(ApiUsageStats::load(&path).unwrap().unresolved_requests(), 1);
    ApiUsageStats::record_response(
        &path,
        Some(&json!({"usage": {
            "prompt_tokens": 678, "completion_tokens": 54, "total_tokens": 732,
            "prompt_cache_hit_tokens": 384,
            "prompt_tokens_details": {"cached_tokens": 384}
        }})),
    );
    ApiUsageStats::record_request(&path);
    ApiUsageStats::record_response(&path, Some(&json!({"usage": null})));
    ApiUsageStats::record_request(&path);
    ApiUsageStats::record_response(&path, None);
    let stats = ApiUsageStats::load(&path).unwrap();
    assert_eq!(stats.requests, 3);
    assert_eq!(stats.responses, 2);
    assert_eq!(stats.failures, 1);
    assert_eq!(stats.unreported_usage, 2);
    assert_eq!(stats.unresolved_requests(), 0);
    assert_eq!(stats.input_tokens, 678);
    assert_eq!(stats.output_tokens, 54);
    assert_eq!(stats.total_tokens, 732);
    assert_eq!(stats.cached_input_tokens, 384);
    assert!(stats.since_unix_secs > 0);
}

#[test]
fn openai_cache_details_and_missing_total_are_supported() {
    let path = scratch();
    ApiUsageStats::record_request(&path);
    ApiUsageStats::record_response(
        &path,
        Some(&json!({"usage": {
            "prompt_tokens": 100, "completion_tokens": 20,
            "prompt_tokens_details": {"cached_tokens": 64}
        }})),
    );
    let stats = ApiUsageStats::load(&path).unwrap();
    assert_eq!(stats.total_tokens, 120);
    assert_eq!(stats.cached_input_tokens, 64);
    assert_eq!(stats.unreported_usage, 0);
    ApiUsageStats::record_request(&path);
    ApiUsageStats::record_response(
        &path,
        Some(&json!({"usage": {"prompt_tokens": -1, "completion_tokens": 20}})),
    );
    let stats = ApiUsageStats::load(&path).unwrap();
    assert_eq!(stats.total_tokens, 120);
    assert_eq!(stats.unreported_usage, 1);
}

#[test]
fn concurrent_writers_merge_without_losing_usage() {
    let path = scratch();
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let path = path.clone();
            std::thread::spawn(move || {
                for _ in 0..5 {
                    ApiUsageStats::record_request(&path);
                    ApiUsageStats::record_response(
                        &path,
                        Some(&json!({"usage": {
                            "prompt_tokens": 10, "completion_tokens": 2, "total_tokens": 12
                        }})),
                    );
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    let stats = ApiUsageStats::load(&path).unwrap();
    assert_eq!(stats.requests, 20);
    assert_eq!(stats.responses, 20);
    assert_eq!(stats.total_tokens, 240);
    assert_eq!(stats.unresolved_requests(), 0);
}

#[test]
fn malformed_statistics_are_preserved_instead_of_reset() {
    let path = scratch();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"{broken").unwrap();
    ApiUsageStats::record_request(&path);
    assert!(ApiUsageStats::load(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"{broken");
}
