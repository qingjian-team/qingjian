//! 自定义短语的编辑：新增或改一条时的草稿（输入码、内容在 `Settings::fields` 里），保存时整张表交给
//! `Config::set_custom_phrases` 校验写回；打开编辑后表被别处改了，保存时不覆盖、草稿留着。

use qingjian_core::CustomPhrase;

/// 编辑中的一条。
#[derive(Debug, Clone)]
pub struct PhraseEditor {
    /// 改的是第几条；`None` 是新增。
    pub index: Option<usize>,

    /// 打开时那一条的样子，保存前比一下有没有被别处改过。
    pub original: Option<CustomPhrase>,

    /// 候选位置 1–9。
    pub position: usize,

    pub enabled: bool,

    /// 保存不了的原因，显示在卡片里。
    pub error: Option<String>,

    /// 「删除」点过一次，等第二次确认。
    pub confirm_delete: bool,
}

impl PhraseEditor {
    pub fn new() -> Self {
        Self {
            index: None,
            original: None,
            position: 1,
            enabled: true,
            error: None,
            confirm_delete: false,
        }
    }

    pub fn edit(index: usize, phrase: &CustomPhrase) -> Self {
        Self {
            index: Some(index),
            original: Some(phrase.clone()),
            position: phrase.position,
            enabled: phrase.enabled,
            error: None,
            confirm_delete: false,
        }
    }

    /// 在 `phrases` 上应用这次编辑（`phrase` 为 `None` 是删除）；表在打开后被改过就报错。
    pub fn apply(
        &self,
        phrases: &[CustomPhrase],
        phrase: Option<CustomPhrase>,
    ) -> Result<Vec<CustomPhrase>, String> {
        let mut list = phrases.to_vec();
        match self.index {
            Some(index) => {
                if list.get(index) != self.original.as_ref() {
                    return Err("这条在别处被改过了，没有保存；关掉再打开看看现在的样子".to_owned());
                }
                match phrase {
                    Some(phrase) => list[index] = phrase,
                    None => {
                        list.remove(index);
                    }
                }
            }
            None => list.extend(phrase),
        }
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phrase(code: &str, position: usize) -> CustomPhrase {
        CustomPhrase {
            code: code.to_owned(),
            text: "x".to_owned(),
            position,
            enabled: true,
        }
    }

    #[test]
    fn edits_refuse_to_overwrite_a_changed_list() {
        let list = vec![phrase("ee", 1), phrase("dz", 1)];
        let editor = PhraseEditor::edit(1, &list[1]);
        let saved = editor.apply(&list, Some(phrase("dz", 2))).unwrap();
        assert_eq!(saved[1].position, 2);
        let changed = vec![phrase("ee", 1), phrase("dz", 3)];
        assert!(editor.apply(&changed, None).is_err());
        assert_eq!(
            PhraseEditor::new()
                .apply(&list, Some(phrase("a", 1)))
                .unwrap()
                .len(),
            3
        );
    }
}
