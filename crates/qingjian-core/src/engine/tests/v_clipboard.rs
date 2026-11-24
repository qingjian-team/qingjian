use super::*;
use qingjian_dictionary::Dictionary;

fn probe() -> Engine {
    Engine::new(Dictionary::parse("开发\tkai fa\t900\n").unwrap())
}

/// 剪切板顶参数这件事缺省关着：用户不知道输入法拿了他复制的东西，剪切板里也可能是密码。
#[test]
fn clipboard_arguments_are_off_by_default() {
    let mut engine = probe();
    engine.set_clipboard_text(Some("1,2,3".to_owned()));
    engine.set_input("vstat");
    assert!(!engine.wants_clipboard(), "缺省不读剪切板");
    assert!(
        engine.query().unwrap().candidates.items.is_empty(),
        "参数空着、又不取剪切板，就没有候选"
    );
}

/// 配置打开之后才用剪切板：`wants_clipboard` 让壳去读一次，候选按剪切板的分隔符连回来。
#[test]
fn clipboard_arguments_follow_the_option() {
    let mut engine = probe();
    engine.set_v_mode_clipboard_args(true);
    engine.set_clipboard_text(Some("1,2,3".to_owned()));
    engine.set_input("vstat");
    assert!(engine.wants_clipboard());
    let texts = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect::<Vec<_>>();
    assert!(
        texts.iter().any(|text| text == "1 2 3"),
        "排序那条：{texts:?}"
    );

    // 关回去时顺手把缓存的那条也丢掉，免得关着还在读
    engine.set_v_mode_clipboard_args(false);
    assert!(!engine.wants_clipboard());
}

/// 打出来的参数优先：剪切板只在参数空着时才有份。
#[test]
fn typed_arguments_win_over_the_clipboard() {
    let mut engine = probe();
    engine.set_v_mode_clipboard_args(true);
    engine.set_clipboard_text(Some("9,9,9".to_owned()));
    engine.set_input("vstat;1;2;5");
    assert!(!engine.wants_clipboard(), "参数已经打出来了");
    let texts = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect::<Vec<_>>();
    assert!(!texts.iter().any(|text| text == "9"), "{texts:?}");
}
