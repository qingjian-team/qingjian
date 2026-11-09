//! 五笔字根图的字形：按字根表（`assets/schemes/wubi86/roots86.tsv`）从思源黑体取轮廓，整字或只留其中几笔，导出成 SVG 路径表。
//!
//! 部件（虎字头、无尾鱼……）大多没有自己的字符，从常用字里取出来，两种手段可以叠用：
//! 挑轮廓（字体把相连的笔画合成一个闭合轮廓，空心也是单独一个，挑的是轮廓编号），分开本来不相连的部分；
//! 加裁剪框（`@左 上 右 下`，字面方块 0–1000），切开连在一起的笔画，渲染时只画框里那一块。
//! 「保留」写 `?` 的还没挑：`--sheet` 画一张轮廓编号图，照着填回字根表。
//! 字体不进仓库（思源黑体 SC Regular，SIL OFL 1.1），导出的路径表随包发，沿用 OFL。

mod outline;
mod sheet;

use std::fmt::Write as _;
use std::path::Path;

use skrifa::FontRef;

use crate::error::ConvertError;
use outline::Outline;

/// 放字根的 25 个键（Z 是万能键，不放字根）。
const KEYS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXY";

pub struct Summary {
    pub roots: usize,

    /// 还没挑笔画（`?`）的部件，按整字导出了。
    pub pending: usize,
}

/// 读字根表，写路径表；`sheet` 给了就把待挑的部件画成编号图。
pub fn run(
    font_path: &Path,
    spec: &Path,
    out: &Path,
    sheet: Option<&Path>,
) -> Result<Summary, ConvertError> {
    let data = std::fs::read(font_path)?;
    let font = FontRef::new(&data).map_err(|error| ConvertError::Format {
        path: font_path.to_owned(),
        line: 0,
        reason: error.to_string(),
    })?;
    let text = std::fs::read_to_string(spec)?;
    let mut table = String::from(
        "# 86 版五笔字根图的字形，由 `dict-convert wubi-roots` 从字根表与思源黑体生成，不要手改。\n\
         # 字形取自思源黑体（Source Han Sans，© Adobe），SIL Open Font License 1.1。\n\
         # 键\t区位\t键名字\t字\t说明\t露出区域（左 上 右 下；空为整字，按字面方块画）\tSVG 路径（字面方块 0–1000，y 向下）\n",
    );
    let mut pending = Vec::new();
    let mut roots = 0;
    let mut seen = String::new();
    for (index, line) in text.lines().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let error = |reason: String| ConvertError::Format {
            path: spec.to_owned(),
            line: index + 1,
            reason,
        };
        let fields: Vec<&str> = line.split('\t').collect();
        let [key, code, name, glyph, strokes, note] = fields[..] else {
            return Err(error(format!("expected 6 columns, got {}", fields.len())));
        };
        if !seen.contains(key) {
            seen.push_str(key);
        }
        let ch = glyph
            .chars()
            .next()
            .ok_or_else(|| error("empty glyph".to_owned()))?;
        let outline =
            Outline::load(&font, ch).ok_or_else(|| error(format!("{ch} is not in the font")))?;
        let (strokes, clip) = match strokes.split_once('@') {
            Some((strokes, clip)) => (strokes, clip.trim()),
            None => (strokes, ""),
        };
        let clip: Vec<i32> = clip
            .split_whitespace()
            .map(|n| {
                n.parse()
                    .map_err(|_| error(format!("bad clip value {n:?}")))
            })
            .collect::<Result<_, _>>()?;
        if !clip.is_empty() && clip.len() != 4 {
            return Err(error(
                "clip needs 4 numbers: left top right bottom".to_owned(),
            ));
        }
        let selected: Option<Vec<usize>> = match strokes.trim() {
            "" => None,
            "?" => {
                // 先按整字导出，渲染不至于缺格
                let path = outline.svg(None);
                let _ = writeln!(table, "{key}\t{code}\t{name}\t{glyph}\t{note}\t\t{path}");
                roots += 1;
                pending.push((format!("{key} {note}（{ch}）"), outline));
                continue;
            }
            list => Some(
                list.split_whitespace()
                    .map(|n| {
                        n.parse()
                            .map_err(|_| error(format!("bad stroke number {n:?}")))
                    })
                    .collect::<Result<_, _>>()?,
            ),
        };
        if let Some(bad) = selected
            .iter()
            .flatten()
            .find(|&&n| n >= outline.contours.len())
        {
            return Err(error(format!(
                "{ch} has {} strokes, no stroke {bad}",
                outline.contours.len()
            )));
        }
        let path = outline.svg(selected.as_deref());
        // 露出区域：整字留空；部件是裁剪框，没写框就是选中几笔的外框
        let frame = match (clip.as_slice(), &selected) {
            ([left, top, right, bottom], _) => format!("{left} {top} {right} {bottom}"),
            (_, Some(keep)) => outline.bounds(Some(keep)).map(|v| v.to_string()).join(" "),
            (_, None) => String::new(),
        };
        let _ = writeln!(
            table,
            "{key}\t{code}\t{name}\t{glyph}\t{note}\t{frame}\t{path}"
        );
        roots += 1;
    }
    let missing: String = KEYS.chars().filter(|key| !seen.contains(*key)).collect();
    if !missing.is_empty() {
        return Err(ConvertError::Format {
            path: spec.to_owned(),
            line: 0,
            reason: format!("no roots for key(s) {missing}"),
        });
    }
    if !pending.is_empty() {
        tracing::warn!(count = pending.len(), "有部件还没挑笔画，按整字导出了");
    }
    std::fs::write(out, table)?;
    if let Some(sheet) = sheet {
        sheet::write(&font, &pending, sheet)?;
        for (i, (label, outline)) in pending.iter().enumerate() {
            println!("{i:>3}  {label}  {} 笔", outline.contours.len());
        }
    }
    Ok(Summary {
        roots,
        pending: pending.len(),
    })
}
