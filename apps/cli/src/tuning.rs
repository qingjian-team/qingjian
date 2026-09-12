//! `--tune 名=值`：把引擎里几组「拍的」常数换成别的值，回放评测时扫参数用；名字见 [`KEYS`]。

use qingjian_core::Engine;
use qingjian_core::correction::TypoCosts;
use qingjian_core::sentence::Interpolation;

/// 可调的参数名。
pub const KEYS: [&str; 10] = [
    "lambda",
    "k",
    "cap",
    "discount",
    "transpose",
    "substitute",
    "extra",
    "missing",
    "typo-cap",
    "correction",
];

/// 把一组 `名=值` 应用到引擎；没给的项保持缺省。
pub fn apply(engine: &mut Engine, settings: &[String]) -> Result<(), TuneError> {
    if settings.is_empty() {
        return Ok(());
    }
    let mut interpolation = Interpolation::default();
    let mut costs = TypoCosts::default();
    for setting in settings {
        let (key, value) = setting
            .split_once('=')
            .ok_or_else(|| TuneError::Syntax(setting.clone()))?;
        let value: f64 = value
            .trim()
            .parse()
            .map_err(|_| TuneError::Value(setting.clone()))?;
        match key.trim() {
            "lambda" => interpolation.lambda = value,
            "k" => interpolation.confidence_k = value,
            "cap" => interpolation.max_confidence = value,
            "discount" => interpolation.trigram_discount = value,
            "transpose" => costs.transpose = value,
            "substitute" => costs.substitute = value,
            "extra" => costs.extra = value,
            "missing" => costs.missing = value,
            "typo-cap" => costs.discount_cap = value,
            "correction" => costs.correction_penalty = value,
            other => {
                return Err(TuneError::Unknown {
                    name: other.to_owned(),
                });
            }
        }
    }
    tracing::info!(?interpolation, ?costs, "参数覆盖");
    engine.set_interpolation(interpolation);
    engine.set_typo_costs(costs);
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum TuneError {
    #[error("--tune expects name=value, got {0:?}")]
    Syntax(String),

    #[error("--tune value is not a number: {0:?}")]
    Value(String),

    #[error("unknown --tune name {name:?}; known: {}", KEYS.join(", "))]
    Unknown { name: String },
}

#[cfg(test)]
mod tests {
    use qingjian_dictionary::Dictionary;

    use super::*;

    fn engine() -> Engine {
        Engine::new(Dictionary::parse("我\two\t100\n").unwrap())
    }

    #[test]
    fn empty_settings_keep_defaults() {
        let mut engine = engine();
        apply(&mut engine, &[]).unwrap();
        assert_eq!(engine.interpolation(), Interpolation::DEFAULT);
        assert_eq!(engine.typo_costs(), TypoCosts::DEFAULT);
    }

    #[test]
    fn settings_override_only_the_named_values() {
        let mut engine = engine();
        apply(
            &mut engine,
            &["lambda=0.7".to_owned(), " substitute = 5.5 ".to_owned()],
        )
        .unwrap();
        let interpolation = engine.interpolation();
        assert_eq!(interpolation.lambda, 0.7);
        assert_eq!(
            interpolation.confidence_k,
            Interpolation::DEFAULT.confidence_k
        );
        let costs = engine.typo_costs();
        assert_eq!(costs.substitute, 5.5);
        assert_eq!(costs.transpose, TypoCosts::DEFAULT.transpose);
    }

    #[test]
    fn rejects_bad_syntax_values_and_names() {
        let mut engine = engine();
        assert!(matches!(
            apply(&mut engine, &["lambda".to_owned()]),
            Err(TuneError::Syntax(_))
        ));
        assert!(matches!(
            apply(&mut engine, &["lambda=abc".to_owned()]),
            Err(TuneError::Value(_))
        ));
        assert!(matches!(
            apply(&mut engine, &["gamma=1".to_owned()]),
            Err(TuneError::Unknown { .. })
        ));
    }
}
