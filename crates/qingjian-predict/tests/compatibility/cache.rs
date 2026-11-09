//! 参数降级与回复格式降级可交错，能力只在同一客户端内复用。

use qingjian_predict::CloudPredictor;
use serde_json::json;

use super::helpers::{prediction, rejection, success, with_server};

#[test]
fn parameter_and_reply_format_fallbacks_are_combined_and_remembered() {
    let format_error = (
        400,
        json!({
            "error": "'response_format.type' must be 'json_schema' or 'text'"
        }),
    );
    let ((), calls) = with_server(
        vec![
            rejection("max_tokens"),
            format_error.clone(),
            rejection("temperature"),
            format_error,
            rejection("reasoning_effort"),
            success(),
            success(),
        ],
        |config| {
            let mut predictor = CloudPredictor::new(config).unwrap();
            assert_eq!(
                prediction(&mut predictor, 1).sentence.as_deref(),
                Some("你好")
            );
            assert_eq!(
                prediction(&mut predictor, 2).sentence.as_deref(),
                Some("你好")
            );
        },
    );
    assert_eq!(calls.len(), 7);
    assert_eq!(calls[0]["max_tokens"], 200);
    for body in &calls[1..] {
        assert!(body.get("max_tokens").is_none());
        assert_eq!(body["max_completion_tokens"], 4296);
    }
    assert_eq!(calls[1]["response_format"]["type"], "json_object");
    assert_eq!(calls[2]["response_format"]["type"], "json_schema");
    assert_eq!(calls[3]["response_format"]["type"], "json_schema");
    for body in &calls[3..] {
        assert!(body.get("temperature").is_none());
    }
    for body in &calls[4..] {
        assert!(body.get("response_format").is_none());
    }
    assert_eq!(calls[4]["reasoning_effort"], "none");
    for body in &calls[5..] {
        assert!(body.get("reasoning_effort").is_none());
    }
    assert_ne!(calls[5]["messages"], calls[6]["messages"]);
}

#[test]
fn independent_clients_do_not_share_rejected_parameters() {
    let ((), calls) = with_server(
        vec![rejection("temperature"), success(), success()],
        |config| {
            let mut first = CloudPredictor::new(config).unwrap();
            assert_eq!(prediction(&mut first, 1).sentence.as_deref(), Some("你好"));
            drop(first);
            let mut second = CloudPredictor::new(config).unwrap();
            assert_eq!(prediction(&mut second, 2).sentence.as_deref(), Some("你好"));
        },
    );
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0]["temperature"], 0.3);
    assert!(calls[1].get("temperature").is_none());
    assert_eq!(calls[2]["temperature"], 0.3);
    assert_eq!(calls[2]["max_tokens"], 200);
    assert_eq!(calls[2]["reasoning_effort"], "none");
}

#[test]
fn legacy_always_thinking_fallback_is_remembered() {
    let thinking_error = (
        400,
        json!({"error": {
            "message": "model always think; cannot disable"
        }}),
    );
    let ((), calls) = with_server(vec![thinking_error, success(), success()], |config| {
        let mut predictor = CloudPredictor::new(config).unwrap();
        assert_eq!(
            prediction(&mut predictor, 1).sentence.as_deref(),
            Some("你好")
        );
        assert_eq!(
            prediction(&mut predictor, 2).sentence.as_deref(),
            Some("你好")
        );
    });
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0]["reasoning_effort"], "none");
    for body in &calls[1..] {
        assert!(body.get("reasoning_effort").is_none());
        assert_eq!(body["max_tokens"], 200);
        assert_eq!(body["temperature"], 0.3);
    }
    assert_ne!(calls[1]["messages"], calls[2]["messages"]);
}
