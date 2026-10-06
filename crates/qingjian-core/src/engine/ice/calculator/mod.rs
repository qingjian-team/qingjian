//! 雾凇算式的有界递归下降解析，不执行 Lua 或任意代码。

mod parser;

pub(super) fn evaluate(input: &str) -> Option<String> {
    if input.len() > 512 || !input.is_ascii() {
        return None;
    }
    let normalized = input.split_whitespace().collect::<String>();
    if let Some(body) = normalized
        .strip_prefix("frexp(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let value = parser::Parser::evaluate(body)?;
        let exponent = if value == 0.0 {
            0.0
        } else {
            value.abs().log2().floor() + 1.0
        };
        let mantissa = value / 2.0_f64.powf(exponent);
        return Some(format!("{mantissa} * 2^{exponent}"));
    }
    let value = parser::Parser::evaluate(&normalized)?;
    value.is_finite().then(|| {
        if value == 0.0 {
            "0".to_owned()
        } else {
            value.to_string()
        }
    })
}

pub(super) fn factorial(n: f64) -> Option<f64> {
    if !(0.0..=170.0).contains(&n) || n.fract() != 0.0 {
        return None;
    }
    Some((1..=n as u32).fold(1.0, |value, i| value * f64::from(i)))
}

pub(super) fn function(name: &str, args: &[f64]) -> Option<f64> {
    let one = || (args.len() == 1).then(|| args[0]);
    let two = || (args.len() == 2).then(|| (args[0], args[1]));
    Some(match name {
        "sin" => one()?.sin(),
        "cos" => one()?.cos(),
        "tan" => one()?.tan(),
        "asin" => one()?.asin(),
        "acos" => one()?.acos(),
        "atan" => one()?.atan(),
        "sinh" => one()?.sinh(),
        "cosh" => one()?.cosh(),
        "tanh" => one()?.tanh(),
        "atan2" => {
            let (a, b) = two()?;
            a.atan2(b)
        }
        "deg" => one()?.to_degrees(),
        "rad" => one()?.to_radians(),
        "exp" => one()?.exp(),
        "sqrt" => one()?.sqrt(),
        "loge" => one()?.ln(),
        "log10" => one()?.log10(),
        "log" => {
            let (base, value) = two()?;
            value.log(base)
        }
        "ldexp" => {
            let (a, b) = two()?;
            a * 2.0_f64.powf(b)
        }
        "fact" => factorial(one()?)?,
        "avg" if !args.is_empty() => args.iter().sum::<f64>() / args.len() as f64,
        "var" if !args.is_empty() => {
            let mean = args.iter().sum::<f64>() / args.len() as f64;
            args.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / args.len() as f64
        }
        "random" | "rdm" if args.len() <= 2 => {
            let bytes = uuid::Uuid::new_v4().into_bytes();
            let r = f64::from(u32::from_le_bytes(bytes[..4].try_into().ok()?)) / 4294967296.0;
            match args {
                [] => r,
                [max] if *max >= 1.0 => (r * max.floor()).floor() + 1.0,
                [min, max] if max >= min => {
                    (r * (max.floor() - min.ceil() + 1.0)).floor() + min.ceil()
                }
                _ => return None,
            }
        }
        _ => return None,
    })
}
