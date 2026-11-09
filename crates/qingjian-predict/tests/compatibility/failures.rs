//! 其他故障不得伪装成参数兼容，也不能无限降级。

use serde_json::json;

use super::helpers::{connection, rejection, with_server};

#[test]
fn authentication_rate_limit_and_server_errors_are_not_retried() {
    for status in [401, 429, 500] {
        let mut error = rejection("temperature");
        error.0 = status;
        let (result, calls) = with_server(vec![error], connection);
        assert!(result.is_err());
        assert_eq!(calls.len(), 1, "status={status}");
        assert_eq!(calls[0]["temperature"], 0.3);
    }
}

#[test]
fn unrelated_validation_and_essential_parameter_errors_are_not_retried() {
    for error in [
        json!({"error": {"message": "max_tokens must be less than remaining context",
            "param": "max_tokens", "code": "invalid_value"}}),
        json!({"error": {"message": "Unsupported value: max_tokens exceeds remaining context",
            "param": "max_tokens", "code": "unsupported_value"}}),
        json!({"error": {"message": "unsupported model", "param": "model",
            "code": "unsupported_parameter"}}),
        json!({"error": {"message": "request could not be validated"}}),
        json!({"error": {"message": "Unsupported parameters: temperature and reasoning_effort"}}),
        json!({"error": {"message": "Unsupported parameter: custom_temperature"}}),
        json!({"error": {"message": "Unsupported temperature", "param": "other_parameter",
            "code": "unsupported_parameter"}}),
    ] {
        let (result, calls) = with_server(vec![(400, error)], connection);
        assert!(result.is_err());
        assert_eq!(calls.len(), 1);
    }
}

#[test]
fn repeating_a_rejection_stops_once_the_field_is_absent() {
    let error = rejection("temperature");
    let (result, calls) = with_server(vec![error.clone(), error], connection);
    assert!(result.is_err());
    assert_eq!(calls.len(), 2);
    assert!(calls[1].get("temperature").is_none());
    assert_eq!(calls[1]["max_tokens"], 32);
}

#[test]
fn rejecting_the_replacement_does_not_remove_the_token_limit() {
    let (result, calls) = with_server(
        vec![rejection("max_tokens"), rejection("max_completion_tokens")],
        connection,
    );
    assert!(result.is_err());
    assert_eq!(calls.len(), 2);
    assert!(calls[1].get("max_tokens").is_none());
    assert_eq!(calls[1]["max_completion_tokens"], 4128);
}
