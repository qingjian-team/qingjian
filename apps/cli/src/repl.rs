//! 交互模式：拼音 → 候选；数字 → 上屏；`:raw` 把上一次输入原样上屏（相当于回车）；`:q` 退出。

use std::io::{self, BufRead, Write};

use qingjian_core::{Engine, Query};

use crate::display;
use crate::error::CliError;

pub fn run(engine: &mut Engine, limit: usize) -> Result<(), CliError> {
    eprintln!(
        "输入拼音查询候选，输入序号上屏，:raw 原样上屏上一次输入（回车），:del N 删掉第 N 个候选，:q 退出。"
    );
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut last: Option<Query> = None;
    loop {
        print!("> ");
        stdout.flush()?;
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == ":q" || line == ":quit" {
            break;
        }
        if let Ok(index) = line.parse::<usize>() {
            commit(engine, last.take(), index);
            continue;
        }
        if line == ":raw" {
            last = None;
            println!("原样上屏: {}", engine.take_raw());
            continue;
        }
        if let Some(index) = line
            .strip_prefix(":del ")
            .and_then(|rest| rest.trim().parse::<usize>().ok())
        {
            forget(engine, last.as_ref(), index);
            continue;
        }
        last = display::show(engine, line, limit);
    }
    Ok(())
}

/// 删掉上一次查询的第 `index` 个（从 1 数）候选：用户词整个删、词库词清学习。
fn forget(engine: &mut Engine, last: Option<&Query>, index: usize) {
    match last.and_then(|q| q.candidates.items.get(index.wrapping_sub(1))) {
        Some(candidate) => {
            let forgotten = engine.forget(candidate);
            println!("删除 {}: {forgotten:?}", candidate.text);
        }
        None => println!("没有第 {index} 个候选"),
    }
}

/// 把上一次查询的第 `index` 个（从 1 数）候选上屏。
fn commit(engine: &mut Engine, last: Option<Query>, index: usize) {
    match last.and_then(|q| q.candidates.items.into_iter().nth(index.wrapping_sub(1))) {
        Some(candidate) => {
            println!("上屏: {}", engine.commit(&candidate));
        }
        None => println!("没有第 {index} 个候选"),
    }
}
