//! IBus 的 GVariant 序列化。
//!
//! IBus 走 D-Bus 传的不是裸字符串，而是它自己那套 `IBusSerializable` 对象：
//! 每个对象序列化成一个元组，头两项固定是**类型名**（`s`）与**附件字典**（`a{sv}`），后面才是各自的字段。
//! 签名错一位不会报错，只会静默不生效（候选窗不出来、preedit 不显示），所以这里的签名由测试钉死。
//!
//! 用到的三种（签名取自 ibus 的 `ibus_*_serialize`）：
//!
//! | 对象 | 签名 | 字段 |
//! |---|---|---|
//! | `IBusText` | `(sa{sv}sv)` | 文本、属性表 |
//! | `IBusAttrList` | `(sa{sv}av)` | 属性数组 |
//! | `IBusAttribute` | `(sa{sv}uuuu)` | 种类、值、起、止 |
//! | `IBusLookupTable` | `(sa{sv}uubbiavav)` | 每页几个、光标、光标可见、循环翻页、朝向、候选、标签 |

use std::collections::HashMap;

use zbus::zvariant::{StructureBuilder, Value};

/// 属性种类：下划线。
const ATTR_UNDERLINE: u32 = 1;

/// 属性种类：前景色。
const ATTR_FOREGROUND: u32 = 2;

/// 下划线样式：细实线，对应 macOS marked text 与 Windows TSF 显示属性里的那一条。
const UNDERLINE_SINGLE: u32 = 1;

/// 候选表朝向：跟随系统面板。
const ORIENTATION_SYSTEM: i32 = 2;

/// 字段类型全是写死的，构造不出来只可能是代码写错，不是运行时条件。
const BUILD_FAILED: &str = "IBus 对象的字段类型是固定的，构造不该失败";

/// 空附件字典：三种对象都不用附件。
fn attachments<'a>() -> HashMap<String, Value<'a>> {
    HashMap::new()
}

/// 一条 `IBusAttribute`：`(sa{sv}uuuu)`。范围按**字符**下标，不是字节。
fn attribute<'a>(kind: u32, value: u32, start: u32, end: u32) -> Value<'a> {
    let structure = StructureBuilder::new()
        .add_field("IBusAttribute".to_owned())
        .add_field(attachments())
        .add_field(kind)
        .add_field(value)
        .add_field(start)
        .add_field(end)
        .build()
        .expect(BUILD_FAILED);
    Value::from(structure)
}

/// `IBusAttrList`：`(sa{sv}av)`。
fn attr_list(attributes: Vec<Value<'static>>) -> Value<'static> {
    let structure = StructureBuilder::new()
        .add_field("IBusAttrList".to_owned())
        .add_field(attachments())
        .add_field(attributes)
        .build()
        .expect(BUILD_FAILED);
    Value::from(structure)
}

/// 一段 preedit 要画的样式。
pub enum Style {
    /// 正在敲的拼音：画细下划线。
    Underline,

    /// 纠错删掉的那一段：IBus 的属性里没有删除线，退而用前景色区分。
    Corrected,
}

/// 带样式的一段文本。
pub struct Segment {
    /// 文本。
    pub text: String,

    /// 这一段怎么画。
    pub style: Style,
}

/// 不带属性的 `IBusText`：上屏文本、候选、辅助文本都用它。
pub fn text(content: &str) -> Value<'static> {
    styled_text(content, Vec::new())
}

/// 分段带样式的 `IBusText`：preedit 用它。属性范围按字符下标算。
pub fn segmented_text(segments: &[Segment]) -> Value<'static> {
    let mut content = String::new();
    let mut attributes = Vec::new();
    let mut start = 0u32;
    for segment in segments {
        let length = segment.text.chars().count() as u32;
        content.push_str(&segment.text);
        let end = start + length;
        match segment.style {
            Style::Underline => {
                attributes.push(attribute(ATTR_UNDERLINE, UNDERLINE_SINGLE, start, end))
            }
            Style::Corrected => {
                attributes.push(attribute(ATTR_UNDERLINE, UNDERLINE_SINGLE, start, end));
                // 0x808080 灰：纠错掉的那一段画浅一点，与 macOS 的删除线是同一个意思
                attributes.push(attribute(ATTR_FOREGROUND, 0x0080_8080, start, end));
            }
        }
        start = end;
    }
    styled_text(&content, attributes)
}

fn styled_text(content: &str, attributes: Vec<Value<'static>>) -> Value<'static> {
    let structure = StructureBuilder::new()
        .add_field("IBusText".to_owned())
        .add_field(attachments())
        .add_field(content.to_owned())
        // 第四项签名是 `v`：要把属性表**包成变体**，直接塞结构体会内联成 `(sa{sv}av)`，签名就错了
        .append_field(Value::new(attr_list(attributes)))
        .build()
        .expect(BUILD_FAILED);
    Value::from(structure)
}

/// `IBusLookupTable`：一页候选。
///
/// **只放当前页**并把 `round` 设成 false：翻页由我们自己算（页大小、云端槽位、跨页高亮都在 Core 的
/// `CandidateLayout` 里），面板只负责把这一页画出来。让面板自己翻页会和 Core 的布局打架。
pub fn lookup_table(candidates: &[String], cursor: u32, visible: bool) -> Value<'static> {
    // `av` 的元素同样要各自包成变体
    let items: Vec<Value<'static>> = candidates.iter().map(|c| Value::new(text(c))).collect();
    let labels: Vec<Value<'static>> = (1..=candidates.len())
        .map(|i| Value::new(text(&i.to_string())))
        .collect();
    let structure = StructureBuilder::new()
        .add_field("IBusLookupTable".to_owned())
        .add_field(attachments())
        .add_field(candidates.len().max(1) as u32)
        .add_field(cursor)
        .add_field(visible)
        .add_field(false)
        .add_field(ORIENTATION_SYSTEM)
        .add_field(items)
        .add_field(labels)
        .build()
        .expect(BUILD_FAILED);
    Value::from(structure)
}

/// 取出结构体的签名字符串，测试用。
#[cfg(test)]
fn signature_of(value: &Value<'_>) -> String {
    value.value_signature().to_string()
}

/// 取出结构体，顺手剥掉外面的变体包装（`av` 的元素与 `v` 的字段都是包着的）。测试用。
#[cfg(test)]
fn as_structure<'a>(value: &'a Value<'a>) -> &'a zbus::zvariant::Structure<'a> {
    match value {
        Value::Structure(structure) => structure,
        Value::Value(inner) => as_structure(inner),
        other => panic!("不是结构体：{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 签名错一位 IBus 就静默不工作，钉死在这里。
    #[test]
    fn ibus_text_has_the_documented_signature() {
        let value = text("你好");
        assert_eq!(signature_of(&value), "(sa{sv}sv)");
    }

    #[test]
    fn attr_list_and_attribute_signatures() {
        let attribute = attribute(ATTR_UNDERLINE, UNDERLINE_SINGLE, 0, 2);
        assert_eq!(signature_of(&attribute), "(sa{sv}uuuu)");
        let list = attr_list(vec![attribute]);
        assert_eq!(signature_of(&list), "(sa{sv}av)");
    }

    #[test]
    fn lookup_table_has_the_documented_signature() {
        let value = lookup_table(&["你好".to_owned(), "你号".to_owned()], 0, true);
        assert_eq!(signature_of(&value), "(sa{sv}uubbiavav)");
    }

    /// 头两项固定是类型名与附件字典。
    #[test]
    fn every_object_starts_with_its_type_name() {
        for (value, name) in [
            (text("x"), "IBusText"),
            (lookup_table(&["x".to_owned()], 0, true), "IBusLookupTable"),
            (attr_list(Vec::new()), "IBusAttrList"),
        ] {
            let fields = as_structure(&value).fields();
            assert_eq!(
                fields[0],
                Value::from(name.to_owned()),
                "第一项该是类型名 {name}"
            );
        }
    }

    /// 属性范围按字符算：中文一个字算一个，不是三个字节。
    #[test]
    fn attribute_ranges_count_characters_not_bytes() {
        let value = segmented_text(&[
            Segment {
                text: "你好".to_owned(),
                style: Style::Underline,
            },
            Segment {
                text: "ma".to_owned(),
                style: Style::Underline,
            },
        ]);
        let fields = as_structure(&value).fields();
        assert_eq!(fields[2], Value::from("你好ma".to_owned()));
        let attrs = as_structure(&fields[3]).fields();
        let Value::Array(list) = &attrs[2] else {
            panic!("属性表该是数组");
        };
        assert_eq!(list.len(), 2);
        // 第二段从第 2 个字符起、到第 4 个字符止
        let second: &Value<'_> = list.get(1).unwrap().unwrap();
        let range = as_structure(second).fields();
        assert_eq!(range[4], Value::U32(2), "起点按字符");
        assert_eq!(range[5], Value::U32(4), "终点按字符");
    }

    /// 每页几个跟着实际候选数走；一个候选都没有时也不能报 0（IBus 会除零）。
    #[test]
    fn empty_lookup_table_still_has_a_page_size() {
        let value = lookup_table(&[], 0, false);
        let fields = as_structure(&value).fields();
        assert_eq!(fields[2], Value::U32(1));
    }

    /// 纠错段多一条前景色属性。
    #[test]
    fn corrected_segment_gets_a_colour_attribute() {
        let value = segmented_text(&[Segment {
            text: "nihao".to_owned(),
            style: Style::Corrected,
        }]);
        let fields = as_structure(&value).fields();
        let attrs = as_structure(&fields[3]).fields();
        let Value::Array(list) = &attrs[2] else {
            panic!("属性表该是数组");
        };
        assert_eq!(list.len(), 2, "下划线加前景色");
    }
}
