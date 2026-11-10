//! 输入串与一种拆法的对齐：逐个部件往下吃，每个部件要么吃它的一个读音，要么吃它笔顺的前几笔。
//!
//! 吃完的部分可以停在任意位置——敲到一半就得出候选，所以 `consumed` 是「吃到哪」而不是「是否吃完」。

use std::borrow::Cow;

use super::table::{ChaiziTable, Hit, Split, Tier};

/// 对齐的中间过程：走到第几个部件、吃掉几个字节、用了几个笔画 token 与读音 token。
#[derive(Clone, Copy)]
struct State {
    part: usize,
    pos: usize,
    strokes: u32,
    readings: u32,

    /// 这条路上有几个部件只吃了一半读音（s 当 si 的前半）。
    partial: u32,
}

/// 一次对齐的结果，按「吃得最多、切得最省、刚好对上」挑。
#[derive(Clone, Copy)]
struct Best {
    pos: usize,
    strokes: u32,
    readings: u32,
    complete: bool,

    /// 有半读的部件：complete 要跟着算假，否则 s+hui 拼出来的 绘 会压过只走了 氵 的 没。
    partial: u32,
}

impl Best {
    fn better_than(self, other: Option<Self>) -> bool {
        let Some(other) = other else {
            return true;
        };
        if self.pos != other.pos {
            return self.pos > other.pos;
        }
        if (self.complete && self.partial == 0) != (other.complete && other.partial == 0) {
            return self.complete && self.partial == 0;
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
            partial: 0,
        },
        &mut best,
    );
    let best = best?;
    // 每个读音都只打了一半（ni 加 ha 拼出的 犗）：太弱，不要。
    // 至少得有一个部件是完整读音（叔 = shang 整 + xi 半 + you 整）。
    if best.partial > 0 && best.partial == best.readings {
        return None;
    }
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
        complete: best.complete && best.partial == 0 && best.pos == body.len(),
        whole: false,
        traditional: split.traditional,
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
        // ü 有两种敲法：表里 j、q、x、y 后写 `u`（`yue`），l、n 后写 `v`（`lve`）；
        // 用户可能敲另一种（`yve`、`lue`），这里两种都认。
        let alt = umlaut_variant(reading);
        let forms = if alt == *reading {
            [Some(*reading), None]
        } else {
            [Some(*reading), Some(alt.as_ref())]
        };
        for form in forms.into_iter().flatten() {
            if rest.starts_with(form) {
                advance(
                    Best {
                        pos: state.pos + form.len(),
                        strokes: state.strokes,
                        readings: state.readings + 1,
                        complete: state.part + 1 == parts.len(),
                        partial: state.partial,
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
                        pos: state.pos + form.len(),
                        strokes: state.strokes,
                        readings: state.readings + 1,
                        partial: state.partial,
                    },
                    best,
                );
            } else if !rest.is_empty() {
                // 读音打了一半：`xi` 对 `xiao`（叔 = 上小又，`ushangxiyou`）。试所有能切的前缀长度，
                // 每个长度都往下走一层——中间部件打一半（小 打成 `xi`）也要能接上后面的 `you`。
                // 读音是 ASCII，按字节切不会破字符边界。
                // 从两个字符起算：一个字母太弱，`unihao` 会因此冒出 犗、犕 这类字。
                let limit = rest.len().min(form.len());
                for cut in 2..=limit {
                    if !form.starts_with(&rest[..cut]) {
                        break;
                    }
                    advance(
                        Best {
                            pos: state.pos + cut,
                            strokes: state.strokes,
                            readings: state.readings + 1,
                            complete: false,
                            partial: state.partial + 1,
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
                            pos: state.pos + cut,
                            strokes: state.strokes,
                            readings: state.readings + 1,
                            partial: state.partial + 1,
                        },
                        best,
                    );
                }
            }
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
                    partial: state.partial,
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
                    partial: state.partial,
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

/// 读音的另一种 ü 敲法：`yue` → `yve`、`juan` → `jvan`、`lve` → `lue`。
///
/// 表里 j、q、x、y 后按《汉语拼音方案》省两点写 `u`，l、n 后写 `v`；
/// 用户两种都可能敲（`yve` 想打 月、`lue` 想打 略），匹配时都认一遍。没有另一种敲法的原样返回。
fn umlaut_variant(reading: &str) -> Cow<'_, str> {
    let first = reading.as_bytes().first().copied();
    match first {
        Some(b'j' | b'q' | b'x' | b'y') => match reading.find('u') {
            Some(at) => Cow::Owned(format!("{}v{}", &reading[..at], &reading[at + 1..])),
            None => Cow::Borrowed(reading),
        },
        Some(b'l' | b'n') if reading.ends_with("ve") => {
            Cow::Owned(format!("{}ue", &reading[..reading.len() - 2]))
        }
        _ => Cow::Borrowed(reading),
    }
}
