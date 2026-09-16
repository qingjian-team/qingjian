//! 系统的文本替换（系统设置「键盘 → 文本替换」）：从全局偏好读出来，Host 把它并进自定义短语。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSUserDefaults};

/// 全局偏好里存文本替换的键（`defaults read -g NSUserDictionaryReplacementItems`）：
/// 每条是 `{ on, replace, with }` 字典，iCloud 同步过来的也在这里。
const REPLACEMENTS_KEY: &str = "NSUserDictionaryReplacementItems";

/// 一条替换：输入码与短语。
pub type TextReplacement = (String, String);

/// 读系统当前启用的文本替换；没设过或结构不认识就是空。内容可能含地址、证件号，不记日志。
pub fn read_system() -> Vec<TextReplacement> {
    let defaults = NSUserDefaults::standardUserDefaults();
    let Some(items) = defaults.arrayForKey(&NSString::from_str(REPLACEMENTS_KEY)) else {
        return Vec::new();
    };
    items.iter().filter_map(|item| parse_item(&item)).collect()
}

/// 一条 `{ on, replace, with }`：`on` 缺省算开；`replace` / `with` 不是字符串的跳过。
fn parse_item(item: &AnyObject) -> Option<TextReplacement> {
    let dict = item.downcast_ref::<NSDictionary>()?;
    let enabled = dict
        .objectForKey(&NSString::from_str("on"))
        .and_then(|value| value.downcast::<NSNumber>().ok())
        .is_none_or(|number| number.as_bool());
    if !enabled {
        return None;
    }
    let replace = string_for(dict, "replace")?;
    let with = string_for(dict, "with")?;
    Some((replace, with))
}

fn string_for(dict: &NSDictionary, key: &str) -> Option<String> {
    let value: Retained<AnyObject> = dict.objectForKey(&NSString::from_str(key))?;
    let text = value.downcast::<NSString>().ok()?;
    Some(text.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_the_global_list_without_panicking() {
        // 这台机器上有没有条目都行；有的话输入码与短语都非空（`on = 0` 的已被过滤）
        let items = super::read_system();
        eprintln!("系统文本替换 {} 条", items.len());
        for (code, text) in &items {
            assert!(!code.is_empty() && !text.is_empty());
        }
    }
}
