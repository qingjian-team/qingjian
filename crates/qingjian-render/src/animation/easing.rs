//! 缓动曲线：CSS 的几个命名曲线与 `cubic-bezier`。主题里写名字或 `[x1, y1, x2, y2]`。

use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) enum Easing {
    Linear,

    #[default]
    Ease,

    EaseIn,

    EaseOut,

    EaseInOut,

    /// 三次贝塞尔的两个控制点 `(x1, y1, x2, y2)`，端点固定在 (0, 0) 与 (1, 1)。
    Bezier([f32; 4]),
}

impl Easing {
    /// 时间进度（0–1）→ 属性进度。
    pub(crate) fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        let points = match self {
            Self::Linear => return t,
            Self::Ease => [0.25, 0.1, 0.25, 1.0],
            Self::EaseIn => [0.42, 0.0, 1.0, 1.0],
            Self::EaseOut => [0.0, 0.0, 0.58, 1.0],
            Self::EaseInOut => [0.42, 0.0, 0.58, 1.0],
            Self::Bezier(points) => points,
        };
        bezier(points, t)
    }
}

/// 解 `x(s) = t` 得参数 `s`（牛顿迭代，不收敛时二分），返回 `y(s)`。
fn bezier([x1, y1, x2, y2]: [f32; 4], t: f32) -> f32 {
    let curve = |a: f32, b: f32, s: f32| {
        let u = 1.0 - s;
        3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
    };
    let slope = |a: f32, b: f32, s: f32| {
        let u = 1.0 - s;
        3.0 * u * u * a + 6.0 * u * s * (b - a) + 3.0 * s * s * (1.0 - b)
    };
    let mut s = t;
    for _ in 0..8 {
        let error = curve(x1, x2, s) - t;
        if error.abs() < 1e-5 {
            return curve(y1, y2, s);
        }
        let d = slope(x1, x2, s);
        if d.abs() < 1e-6 {
            break;
        }
        s = (s - error / d).clamp(0.0, 1.0);
    }
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    s = t;
    for _ in 0..30 {
        if curve(x1, x2, s) < t {
            low = s;
        } else {
            high = s;
        }
        s = (low + high) / 2.0;
    }
    curve(y1, y2, s)
}

impl<'de> Deserialize<'de> for Easing {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Name(String),
            Points([f32; 4]),
        }
        match Raw::deserialize(deserializer)? {
            Raw::Name(name) => match name.as_str() {
                "linear" => Ok(Self::Linear),
                "ease" => Ok(Self::Ease),
                "ease-in" => Ok(Self::EaseIn),
                "ease-out" => Ok(Self::EaseOut),
                "ease-in-out" => Ok(Self::EaseInOut),
                _ => Err(serde::de::Error::custom(format!(
                    "不认识的缓动曲线 {name:?}"
                ))),
            },
            Raw::Points([x1, y1, x2, y2])
                if (0.0..=1.0).contains(&x1) && (0.0..=1.0).contains(&x2) =>
            {
                Ok(Self::Bezier([x1, y1, x2, y2]))
            }
            Raw::Points(_) => Err(serde::de::Error::custom("贝塞尔控制点的 x 要在 0–1 之内")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_and_monotonic() {
        for easing in [
            Easing::Linear,
            Easing::Ease,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
        ] {
            assert!(easing.apply(0.0).abs() < 1e-3);
            assert!((easing.apply(1.0) - 1.0).abs() < 1e-3);
            let samples: Vec<f32> = (0..=20).map(|i| easing.apply(i as f32 / 20.0)).collect();
            assert!(samples.windows(2).all(|pair| pair[1] >= pair[0] - 1e-4));
        }
        // ease-out 前快后慢
        assert!(Easing::EaseOut.apply(0.5) > 0.5);
        assert!(Easing::EaseIn.apply(0.5) < 0.5);
    }

    #[test]
    fn parses_names_and_points() {
        let parse = |json: &str| serde_json::from_str::<Easing>(json);
        assert_eq!(parse("\"ease-out\"").unwrap(), Easing::EaseOut);
        assert_eq!(
            parse("[0.2, 0, 0, 1]").unwrap(),
            Easing::Bezier([0.2, 0.0, 0.0, 1.0])
        );
        assert!(parse("\"bounce\"").is_err());
        assert!(parse("[2, 0, 0, 1]").is_err());
    }
}
