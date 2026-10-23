//! 本机验证入口：模型目录、运行库路径、目标语言与待翻译选区。

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err("usage: translate MODELS RUNTIME TARGET TEXT".into());
    }
    let direction = match args[2].as_str() {
        "en" => "zh-en",
        "zh" => "en-zh",
        _ => return Err("unsupported target".into()),
    };
    let start = std::time::Instant::now();
    let mut backend = qingjian_offline::Backend::load(
        &PathBuf::from(&args[0]).join(direction),
        &PathBuf::from(&args[1]),
    )?;
    let translated = backend.translate(&args[3], &args[2], || false)?;
    println!("{translated}");
    eprintln!("elapsed: {:?}", start.elapsed());
    Ok(())
}
