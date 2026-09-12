//! TSV → `.qj`：解析成内存结构后原样落盘，加上元数据；`model` 是三件套目录 → `.qjm`。

use std::path::{Path, PathBuf};
use std::time::Instant;

use qingjian_core::Language;
use qingjian_dictionary::Dictionary;
use qingjian_format::Metadata;
use qingjian_lm::BigramModel;
use qingjian_translate::Glossary;

use crate::args::PackKind;
use crate::error::ConvertError;

/// 打包一种数据。`inputs` 为空时从 `out_dir` 里找缺省的 TSV。
pub fn pack(
    kind: PackKind,
    inputs: &[PathBuf],
    language: &str,
    metadata: Metadata,
    out_dir: &Path,
) -> Result<(), ConvertError> {
    let metadata = Metadata {
        generator: format!("qingjian-dict-convert {}", env!("CARGO_PKG_VERSION")),
        ..metadata
    };
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
                reason: "language must be en / ja / zh".to_owned(),
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
    }
    Ok(())
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
