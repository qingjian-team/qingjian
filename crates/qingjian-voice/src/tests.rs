//! 配置与识别器在不联网时的行为；协议与请求细节在 `volcengine` 内部的测试里。

use qingjian_core::Transcriber;

use crate::{VolcengineConfig, VolcengineTranscriber};

#[test]
fn new_trims_the_key_and_resource_id() {
    let config = VolcengineConfig::new("  key  ", " volc.seedasr.sauc.duration ");
    assert_eq!(config.api_key, "key");
    assert_eq!(config.resource_id, "volc.seedasr.sauc.duration");
}

#[test]
fn new_defaults_to_the_streaming_endpoint_and_punctuation() {
    let config = VolcengineConfig::new("key", VolcengineConfig::HOURLY);
    assert_eq!(config.endpoint, VolcengineConfig::ENDPOINT);
    assert!(config.punctuation);
}

#[test]
fn ready_needs_both_fields() {
    assert!(!VolcengineConfig::new("", VolcengineConfig::HOURLY).ready());
    assert!(!VolcengineConfig::new("key", "").ready());
    assert!(!VolcengineConfig::new("  ", " \t ").ready());
    assert!(VolcengineConfig::new("key", VolcengineConfig::CONCURRENT).ready());
}

#[test]
fn billing_modes_have_their_own_ids() {
    assert_eq!(VolcengineConfig::HOURLY, "volc.seedasr.sauc.duration");
    assert_eq!(VolcengineConfig::CONCURRENT, "volc.seedasr.sauc.concurrent");
    assert_eq!(
        VolcengineConfig::ENDPOINT,
        "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_async"
    );
}

#[test]
fn transcriber_without_credentials_is_unavailable() {
    let transcriber = VolcengineTranscriber::new(VolcengineConfig::new("", ""));
    assert!(!transcriber.available());
}

#[test]
fn feeding_before_start_is_dropped() {
    let mut transcriber =
        VolcengineTranscriber::new(VolcengineConfig::new("key", VolcengineConfig::HOURLY));
    assert!(transcriber.available());
    transcriber.feed(&[1, 2, 3]);
    transcriber.finish();
    transcriber.cancel();
    assert!(transcriber.poll().is_none());
}
