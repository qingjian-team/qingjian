//! 语音识别的配置：设置页「语音识别」那份（与云端大模型分开），现在只有火山引擎豆包流式语音识别。
//! 密钥同云端那份一样由 Kotlin 从 Android Keystore 解出来交过来，只放在内存里。

mod settings;

pub use settings::VoiceSettings;

/// 设置页「测试连接」：阻塞到有结果，Kotlin 在后台线程调。成功回 `Ok(提示)`，失败回 `Err(给用户看的原因)`。
pub fn test_voice(settings: &VoiceSettings) -> Result<String, String> {
    let config = settings
        .volcengine()
        .ok_or_else(|| "还没选识别服务".to_owned())?;
    let elapsed = qingjian_voice::volcengine::test_connection(&config)?;
    Ok(format!("连上了，用时 {} 毫秒", elapsed.as_millis()))
}
