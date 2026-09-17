//! 松键记账测试：按下被吞的键，松键也要吞；透传的松键照样透传。

use super::*;

#[test]
fn releases_swallowed_iff_press_was_swallowed() {
    // 上屏类的键按下就结束组句：松键仍须吞，否则应用收到无头 keyup。
    for (name, keyval) in [
        ("空格", 0x20u32),
        ("回车", 0xff0d),
        ("Esc", 0xff1b),
        ("数字1", 0x31),
    ] {
        let mut h = sample_host();
        type_str(&mut h, "ni");
        assert!(h.key(keyval, 0, false), "{name} 按下应被吞");
        assert!(!h.composing(), "{name} 按下后组句应已结束");
        assert!(h.key(keyval, 0, true), "{name} 松键应与按下对称地被吞");
    }
    // 空闲时敲逗号转全角上屏：按下吞，松键同吞。
    let mut h = sample_host();
    assert!(h.key(0x2c, 0, false), "空闲逗号按下应被吞(转全角)");
    assert!(h.pending_commit.take().is_some());
    assert!(h.key(0x2c, 0, true), "空闲逗号松键应被吞");
}

#[test]
fn rollover_releases_after_commit_are_swallowed() {
    // 按住字母未松就敲空格上屏(rollover)：字母的松键到达时组句已结束，按下是我们吞的，松键也吞。
    let mut h = sample_host();
    type_str(&mut h, "ni");
    assert!(h.key(0x20, 0, false));
    assert!(!h.composing());
    assert!(h.key(0x69, 0, true), "i 的按下被吞过,松键也吞");
    assert!(h.key(0x6e, 0, true), "n 同理");
    assert!(!h.key(0x78, 0, true), "x 没按过,松键透传");
    h.pending_commit.take();
}

#[test]
fn passthrough_presses_keep_their_releases_passthrough() {
    // 按下透传过的键，松键也透传（应用见过按下，吞掉松键同样制造不对称）。
    let mut h = sample_host();
    // 空闲敲 ]:punctuate 转不动，透传；随后组句中它的松键到达，也必须透传。
    let press = h.key(0x5d, 0, false);
    type_str(&mut h, "ni");
    if !press {
        assert!(!h.key(0x5d, 0, true), "按下透传过的 ] 组句中松键也透传");
    }
    // Ctrl+a：按下透传，松键透传。
    assert!(!h.key(0x61, 1 << 2, false), "Ctrl+a 按下透传");
    assert!(!h.key(0x61, 1 << 2, true), "Ctrl+a 松键透传");
    // 同一键的松键只吞一次：上屏空格的松键吞过之后，再来一次（没有对应按下）就透传。
    let mut h = sample_host();
    type_str(&mut h, "ni");
    assert!(h.key(0x20, 0, false));
    assert!(h.key(0x20, 0, true));
    assert!(!h.key(0x20, 0, true), "没有对应按下的第二次松键透传");
    h.pending_commit.take();
}

#[test]
fn shifted_digit_release_swallowed_across_keysym_drift() {
    // 删候选快捷键（缺省 Shift+数字）按下到达的是符号 keysym(!)，先松 Shift 再松键时
    // 松键到达的是数字 keysym(1)：同一物理键的两个 keysym 记账要互认。
    let mut h = sample_host();
    type_str(&mut h, "ni");
    assert!(h.key(0x21, 1 << 0, false), "Shift+1(!)按下应被删候选键吞");
    assert!(
        h.key(0x31, 0, true),
        "先松 Shift 后的数字松键(keysym=1)也应被吞"
    );
    assert!(!h.key(0x31, 0, true), "账已消,再来的孤松键透传");
    assert!(!h.key(0x21, 0, true), "符号侧也不留陈账");
    // 反向：Shift 一直按着，松键仍是符号 keysym，同样命中。
    let mut h = sample_host();
    type_str(&mut h, "ni");
    assert!(h.key(0x21, 1 << 0, false));
    assert!(
        h.key(0x21, 1 << 0, true),
        "Shift 未松时符号 keysym 松键命中本账"
    );
    // 陈账清理也认变体：！ 的账挂着（松键丢失），之后数字 1 的按下透传时应连带清掉。
    let mut h = sample_host();
    type_str(&mut h, "ni");
    assert!(h.key(0x21, 1 << 0, false), "挂一笔 ! 的账");
    h.key(0xff1b, 0, false); // Esc 清组句（其账随即被消：此处只关心 ！ 的）
    h.key(0xff1b, 0, true);
    assert!(!h.key(0x31, 0, false), "空闲数字按下透传(半角规则)");
    assert!(
        !h.key(0x31, 0, true),
        "透传按下连带清掉 ! 陈账,松键不被误吞"
    );
    assert!(!h.key(0x21, 0, true), "! 的陈账确实没了");
}
