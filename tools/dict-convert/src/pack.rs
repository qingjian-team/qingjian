//! TSV → `.qj`：解析成内存结构后原样落盘，加上元数据；`model` 是三件套目录 → `.qjm`，
//! `codes` 是唯一带计算的一种（笔画表 + 词库 → 码表，见 `codes` 模块）。

use std::path::{Path, PathBuf};
use std::time::Instant;

use qingjian_core::Language;
use qingjian_dictionary::Dictionary;
use qingjian_format::Metadata;
use qingjian_lm::BigramModel;
use qingjian_translate::Glossary;

use crate::args::PackKind;
use crate::error::ConvertError;

/// 随包笔画码表的名称（`pack codes` 的元数据缺省值）。
const CODES_NAME: &str = "笔画";

/// 许可：CNS11643 筆順資料开放数据「政府資料開放授權條款第 1 版 / OFL-1.1」二选一，取 OFL-1.1。
const CODES_LICENSE: &str = "OFL-1.1";

/// 署名：与 `assets/stroke/README.md` 的「随包时的许可合规」一致。
const CODES_ATTRIBUTION: &str = "CNS11643 全字庫筆順資料（中華民國數位發展部）";

/// 来源：全字庫开放数据的数据集页。
const CODES_SOURCE: &str = "https://data.gov.tw/dataset/5961";

/// `pack codes` 的三个路径（别的种类不给）。都不给时都从 `out_dir` 里找。
#[derive(Debug, Default)]
pub struct CodePaths<'a> {
    /// 笔画表（`stroke` 子命令的产物）。
    pub stroke: Option<&'a Path>,

    /// 取码用的词库。
    pub dict: Option<&'a Path>,

    /// 码表产物。
    pub output: Option<&'a Path>,
}

/// 打包一种数据。`inputs` 为空时从 `out_dir` 里找缺省的 TSV。
pub fn pack(
    kind: PackKind,
    inputs: &[PathBuf],
    paths: &CodePaths<'_>,
    language: &str,
    metadata: Metadata,
    out_dir: &Path,
) -> Result<(), ConvertError> {
    let metadata = Metadata {
        generator: format!("qingjian-dict-convert {}", env!("CARGO_PKG_VERSION")),
        ..metadata
    };
    // 只有 codes 自己带元数据缺省值（名称、许可、署名都是产品决定），别的种类仍然要显式给。
    if kind != PackKind::Codes && metadata.name.is_empty() {
        return Err(ConvertError::MissingName { kind: kind.name() });
    }
    let started = Instant::now();
    match kind {
        PackKind::Dict => {
            let input = inputs
                .first()
                .cloned()
                .unwrap_or_else(|| out_dir.join("dict.tsv"));
            let dictionary = Dictionary::from_path(&input)?;
            let out = out_dir.join("dict.qj");
            dictionary.write_qj(&out, &metadata)?;
            report(&out, dictionary.len(), started);
        }
        PackKind::Lm => {
            let (unigram, bigram) = match inputs {
                [unigram, bigram, ..] => (unigram.clone(), bigram.clone()),
                _ => (
                    out_dir.join("lm-unigram.tsv"),
                    out_dir.join("lm-bigram.tsv"),
                ),
            };
            let model = BigramModel::from_paths(&unigram, &bigram)?;
            let out = out_dir.join("lm.qj");
            model.write_qj(&out, &metadata)?;
            report(&out, model.bigram_count(), started);
        }
        PackKind::Glossary => {
            let language: Language = language.parse().map_err(|_| ConvertError::Format {
                path: PathBuf::from(language),
                line: 0,
                reason: "language must be en / ja / zh / es".to_owned(),
            })?;
            let input = inputs.first().cloned().unwrap_or_else(|| {
                PathBuf::from("assets/glossary").join(format!("glossary-{}.tsv", language.code()))
            });
            let glossary = Glossary::from_path(language, &input)?;
            let out = out_dir.join(format!("glossary-{}.qj", language.code()));
            glossary.write_qj(&out, &metadata)?;
            report(&out, glossary.len(), started);
        }
        PackKind::Model => {
            let input = inputs
                .first()
                .cloned()
                .unwrap_or_else(|| PathBuf::from("data/model"));
            let out = out_dir.join("model.qjm");
            let parameters = qingjian_neural::qjm::pack(&input, &out, &metadata)?;
            report(
                &out,
                usize::try_from(parameters).unwrap_or(usize::MAX),
                started,
            );
        }
        PackKind::Codes => {
            let stroke = paths
                .stroke
                .map(Path::to_path_buf)
                .unwrap_or_else(|| out_dir.join("codes").join("stroke.tsv"));
            let dict = paths
                .dict
                .map(Path::to_path_buf)
                .unwrap_or_else(|| out_dir.join("dict.qj"));
            let out = paths
                .output
                .map(Path::to_path_buf)
                .unwrap_or_else(|| out_dir.join("codes").join("stroke.qj"));
            crate::codes::build(&stroke, &dict, &out, &codes_metadata(metadata, &stroke))?;
        }
    }
    Ok(())
}

/// `pack codes` 的元数据缺省值：名称、许可、署名、来源都是这份产品数据的决定
/// （许可依据见 `docs/design/aux-code.md` 的「许可」一节），调用方不用重复给，给了的以给的为准。
fn codes_metadata(mut metadata: Metadata, stroke: &Path) -> Metadata {
    if metadata.name.is_empty() {
        metadata.name = CODES_NAME.to_owned();
    }
    if metadata.license.is_empty() {
        metadata.license = CODES_LICENSE.to_owned();
    }
    if metadata.attribution.is_empty() {
        metadata.attribution = CODES_ATTRIBUTION.to_owned();
    }
    if metadata.source.is_empty() {
        metadata.source = CODES_SOURCE.to_owned();
    }
    if metadata.version.is_empty() {
        // 上游没有版本号：取笔画表（CNS 筆順資料的快照）的日期当数据版本。
        metadata.version = source_date(stroke)
            .map(|date| date.to_string())
            .unwrap_or_default();
    }
    metadata
}

/// 文件的日期（本地时区）；拿不到时 `None`。
fn source_date(path: &Path) -> Option<jiff::civil::Date> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let seconds = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    let stamp = jiff::Timestamp::from_second(i64::try_from(seconds).ok()?).ok()?;
    Some(stamp.to_zoned(jiff::tz::TimeZone::system()).date())
}

fn report(out: &Path, entries: usize, started: Instant) {
    let size = std::fs::metadata(out).map(|m| m.len()).unwrap_or(0);
    tracing::info!(
        out = %out.display(),
        entries,
        size_mb = size / 1_000_000,
        elapsed_ms = started.elapsed().as_millis(),
        "已打包"
    );
}
