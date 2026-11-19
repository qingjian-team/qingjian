//! 本机地址上的服务（LM Studio、自己起的 llama-server）不要密钥。

use qingjian_predict::PredictConfig;

use super::helpers::{connection, success, with_server};

#[test]
fn local_services_are_reached_without_a_key() {
    let (result, calls) = with_server(vec![success()], |config| {
        connection(&PredictConfig {
            api_key: None,
            api_key_env: "QINGJIAN_TEST_KEY_THAT_IS_NEVER_SET".to_owned(),
            ..config.clone()
        })
    });
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(calls.len(), 1);
}
