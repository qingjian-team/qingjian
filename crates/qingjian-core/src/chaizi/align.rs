//! 输入串与一种拆法的对齐：逐个部件往下吃，每个部件要么吃它的一个读音，要么吃它笔顺的前几笔。
//!
//! 吃完的部分可以停在任意位置——敲到一半就得出候选，所以 `consumed` 是「吃到哪」而不是「是否吃完」。

use super::table::{ChaiziTable, Hit, Split, Tier};

/// 对齐的中间过程：走到第几个部件、吃掉几个字节、用了几个笔画 token 与读音 token。
#[derive(Clone, Copy)]
struct State {
    part: usize,
    pos: usize,
    strokes: u32,
    readings: u32,
}

/// 一次对齐的结果，按「吃得最多、切得最省、刚好对上」挑。
#[derive(Clone, Copy)]
struct Best {
    pos: usize,
    strokes: u32,
    readings: u32,
    complete: bool,
}

impl Best {
    fn better_than(self, other: Option<Self>) -> bool {
        let Some(other) = other else {
            return true;
        };
        if self.pos != other.pos {
            return self.pos > other.pos;
        }
        if self.complete != other.complete {
            return self.complete;
        }
        self.strokes + self.readings < other.strokes + other.readings
    }
}

/// 拿 `body` 去对一条拆法，对不上返回 `None`。
pub(super) fn align(table: &ChaiziTable, split: &Split, body: &str) -> Option<Hit> {
    let parts: Vec<char> = split.parts.chars().collect();
    let mut readings: Vec<Vec<&'static str>> = Vec::with_capacity(parts.len());
    for (index, part) in parts.iter().enumerate() {
        let mut list: Vec<&'static str> = split
            .readings
            .split('\'')
            .nth(index)
            .unwrap_or_default()
            .split(',')
            .filter(|one| !one.is_empty())
            .collect();
        for extra in table.alias_readings(*part) {
            if !list.contains(extra) {
                list.push(extra);
            }
        }
        readings.push(list);
    }

    let mut best: Option<Best> = None;
    walk(
        table,
        &parts,
        &readings,
        body,
        State {
            part: 0,
            pos: 0,
            strokes: 0,
            readings: 0,
        },
        &mut best,
    );
    let best = best?;
    if best.pos == 0 {
        return None;
    }
    let tier = if best.strokes == 0 {
        Tier::Reading
    } else if best.readings == 0 {
        Tier::Stroke
    } else {
        Tier::Mixed
    };
    Some(Hit {
        ch: table.char_of(split.entry),
        reading: table.reading_of(split.entry),
        tier,
        consumed: best.pos,
        remaining: 0,
        complete: best.complete && best.pos == body.len(),
        whole: false,
    })
}

fn walk(
    table: &ChaiziTable,
    parts: &[char],
    readings: &[Vec<&'static str>],
    body: &str,
    state: State,
    best: &mut Option<Best>,
) {
    if state.part == parts.len() {
        return;
    }
    let rest = &body[state.pos..];
    for reading in &readings[state.part] {
        if rest.starts_with(*reading) {
            advance(
                Best {
                    pos: state.pos + reading.len(),
                    strokes: state.strokes,
                    readings: state.readings + 1,
                    complete: state.part + 1 == parts.len(),
                },
                state,
                best,
            );
            walk(
                table,
                parts,
                readings,
                body,
                State {
                    part: state.part + 1,
                    pos: state.pos + reading.len(),
                    strokes: state.strokes,
                    readings: state.readings + 1,
                },
                best,
            );
        }
    }
    if let Some(sequence) = table.strokes_of(parts[state.part]) {
        let mut eaten = 0;
        for (offset, expected) in sequence.char_indices() {
            let Some(actual) = rest.get(offset..).and_then(|tail| tail.chars().next()) else {
                break;
            };
            if actual != expected {
                break;
            }
            eaten = offset + expected.len_utf8();
            advance(
                Best {
                    pos: state.pos + eaten,
                    strokes: state.strokes + 1,
                    readings: state.readings,
                    complete: state.part + 1 == parts.len(),
                },
                state,
                best,
            );
        }
        if eaten > 0 {
            walk(
                table,
                parts,
                readings,
                body,
                State {
                    part: state.part + 1,
                    pos: state.pos + eaten,
                    strokes: state.strokes + 1,
                    readings: state.readings,
                },
                best,
            );
        }
    }
}

/// 记下一个候选（只在这里比较，`State` 本身不参与排序）。
fn advance(candidate: Best, _state: State, best: &mut Option<Best>) {
    if candidate.better_than(*best) {
        *best = Some(candidate);
    }
}
