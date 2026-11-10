//! 在显式启用的隔离五笔夹具中验证真实 IBus 信号。

use std::time::Duration;
use zbus::connection::Builder;

use super::{IBusProxy, InputContextProxy, SIGNAL_TIMEOUT, open_context};

#[tokio::test(flavor = "multi_thread")]
async fn wubi_commits_and_keeps_next_preedit() {
    if std::env::var_os("QINGJIAN_IBUS_WUBI_TEST").is_none() {
        return;
    }
    tokio::time::timeout(Duration::from_secs(45), exercise_wubi())
        .await
        .expect("隔离 IBus 五笔测试超时");
}

async fn exercise_wubi() {
    let address = std::env::var("IBUS_ADDRESS").expect("必须提供隔离 IBus 总线");
    let connection = Builder::address(address.as_str())
        .unwrap()
        .build()
        .await
        .unwrap();
    let bus = IBusProxy::new(&connection).await.unwrap();
    let (context, signals) = open_context(&connection, &bus, "qingjian-wubi").await;
    if context.set_engine("qingjian").await.is_err() {
        bus.set_global_engine("qingjian").await.unwrap();
    }
    context.focus_in().await.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    type_keys(&context, "ga").await;
    assert!(
        signals.wait_for("UpdateLookupTable", "开").await.is_some(),
        "{}",
        signals.summary()
    );
    type_keys(&context, " ").await;
    assert!(
        signals.wait_for("CommitText", "开").await.is_some(),
        "{}",
        signals.summary()
    );
    type_keys(&context, "ggll").await;
    assert!(
        signals.wait_for("CommitText", "一").await.is_some(),
        "{}",
        signals.summary()
    );
    type_keys(&context, "aaaa").await;
    assert!(context.process_key_event(0xff54, 116, 0).await.unwrap());
    type_keys(&context, "g").await;
    assert!(
        signals.wait_for("CommitText", "乙").await.is_some(),
        "{}",
        signals.summary()
    );
    let deadline = tokio::time::Instant::now() + SIGNAL_TIMEOUT;
    loop {
        let seen = signals.seen.lock().unwrap().clone();
        if let Some(commit) = seen
            .iter()
            .position(|(name, text)| name == "CommitText" && text.contains('乙'))
            && seen[commit + 1..]
                .iter()
                .any(|(name, text)| name == "UpdatePreeditText" && text.contains("\"g\""))
        {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "顶字后应保留新编码：{}",
            signals.summary()
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    type_keys(&context, "a ").await;
    context.focus_out().await.unwrap();
}

async fn type_keys(context: &InputContextProxy<'_>, text: &str) {
    for character in text.chars() {
        let keycode = match character {
            'g' => 42,
            'a' => 38,
            'l' => 46,
            ' ' => 65,
            _ => unreachable!(),
        };
        assert!(
            context
                .process_key_event(character as u32, keycode, 0)
                .await
                .unwrap()
        );
    }
}
