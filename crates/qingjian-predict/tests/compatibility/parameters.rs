//! 参数兼容由接口拒绝驱动，并在同一客户端的后续请求中记住。

use qingjian_predict::CloudPredictor;
use serde_json::json;

use super::helpers::{connection, prediction, rejection, success, with_server};

#[test]
fn supported_parameters_are_not_changed_by_the_model_name() {
    for model in ["gpt-6.1-sol", "vendor/unlisted-model"] {
        let (result, calls) = with_server(vec![success()], |config| {
            let mut config = config.clone();
            config.model = model.to_owned();
            connection(&config)
        });
        result.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["model"], model);
        assert_eq!(calls[0]["max_tokens"], 32);
        assert_eq!(calls[0]["temperature"], 0.3);
        assert_eq!(calls[0]["reasoning_effort"], "none");
        assert!(calls[0].get("max_completion_tokens").is_none());
    }
}

#[test]
fn connection_renames_rejected_token_limit_and_keeps_a_finite_budget() {
    let (result, calls) = with_server(vec![rejection("max_tokens"), success()], connection);
    result.unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0]["max_tokens"], 32);
    assert!(calls[1].get("max_tokens").is_none());
    assert_eq!(calls[1]["max_completion_tokens"], 4128);
    assert_eq!(calls[1]["temperature"], 0.3);
    assert_eq!(calls[1]["reasoning_effort"], "none");
}

#[test]
fn prediction_learns_each_rejection_and_reuses_the_result() {
    for model in ["new-reasoning-model", "another-vendor/model"] {
        let reasoning = (
            400,
            json!({"error": {
                "message": "Invalid option: expected one of low, medium, high",
                "param": "reasoning_effort", "code": "invalid_value"
            }}),
        );
        let ((), calls) = with_server(
            vec![
                rejection("max_tokens"),
                rejection("temperature"),
                reasoning,
                success(),
                success(),
            ],
            |config| {
                let mut config = config.clone();
                config.model = model.to_owned();
                let mut predictor = CloudPredictor::new(&config).unwrap();
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
        assert_eq!(calls.len(), 5);
        assert_eq!(calls[0]["model"], model);
        assert_eq!(calls[0]["max_tokens"], 200);
        assert_eq!(calls[0]["temperature"], 0.3);
        assert_eq!(calls[0]["reasoning_effort"], "none");
        for body in &calls[1..] {
            assert!(body.get("max_tokens").is_none());
            assert_eq!(body["max_completion_tokens"], 4296);
            assert_eq!(body["response_format"]["type"], "json_object");
        }
        assert_eq!(calls[1]["temperature"], 0.3);
        assert_eq!(calls[2]["reasoning_effort"], "none");
        for body in &calls[2..] {
            assert!(body.get("temperature").is_none());
        }
        for body in &calls[3..] {
            assert!(body.get("reasoning_effort").is_none());
        }
        assert_ne!(calls[3]["messages"], calls[4]["messages"]);
    }
}

#[test]
fn explicit_message_without_param_only_removes_the_rejected_field() {
    let error = (
        400,
        json!({"error": {
            "message": "Unsupported parameter: 'temperature' is not supported with this model."
        }}),
    );
    let (result, calls) = with_server(vec![error, success()], connection);
    result.unwrap();
    assert_eq!(calls.len(), 2);
    assert!(calls[1].get("temperature").is_none());
    assert_eq!(calls[1]["max_tokens"], 32);
    assert_eq!(calls[1]["reasoning_effort"], "none");
    assert_eq!(calls[1]["response_format"], calls[0]["response_format"]);
}

#[test]
fn structured_param_takes_priority_over_other_fields_in_the_message() {
    let error = (
        422,
        json!({"error": {
            "message": "Unsupported temperature; reasoning_effort remains available",
            "param": "temperature", "code": "unsupported_parameter"
        }}),
    );
    let (result, calls) = with_server(vec![error, success()], connection);
    result.unwrap();
    assert_eq!(calls.len(), 2);
    assert!(calls[1].get("temperature").is_none());
    assert_eq!(calls[1]["reasoning_effort"], "none");
    assert_eq!(calls[1]["max_tokens"], 32);
}
