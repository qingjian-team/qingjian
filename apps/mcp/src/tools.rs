//! MCP 工具：`lookup`（拼音 → 候选 + 释义）与 `gloss`（词 → 译文）。
//! 两条都走 Core 的真实链路（`Engine::query` / `Engine::annotate`），结果与输入法 / CLI 一致。

use std::cell::RefCell;

use qingjian_core::{Candidate, CandidateKind, CandidateList, Engine, FuzzyRules, Query};
use serde_json::{Value, json};

use crate::error::Error;

/// MCP 服务器持有的引擎；请求串行处理，`RefCell` 满足 `query` 的 `&self` 与 `set_input` 的 `&mut`。
pub struct Tools {
    engine: RefCell<Engine>,
}

impl Tools {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine: RefCell::new(engine),
        }
    }

    /// `tools/list` 的工具清单（JSON Schema）。
    pub fn list() -> Value {
        json!({
            "tools": [
                {
                    "name": "lookup",
                    "description": "按拼音输入串查青简候选词（与输入法同一排序与释义），返回切分、纠正与候选列表",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "input": { "type": "string", "description": "拼音输入串，如 ni'hao；可带 | 表示光标停在中间" },
                            "limit": { "type": "integer", "minimum": 1, "maximum": 50, "description": "最多返回的候选数（缺省 10）" },
                            "fuzzy": { "type": "string", "description": "模糊音，逗号分隔：z-zh,c-ch,s-sh,n-l,f-h,l-r,an-ang,en-eng,in-ing；all 全开" }
                        },
                        "required": ["input"]
                    }
                },
                {
                    "name": "gloss",
                    "description": "查一个词的学习语言译文（启动时的 --language，缺省 en），与输入法候选窗显示的释义同源",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "word": { "type": "string", "description": "要查的词，如 青简 或 hello" }
                        },
                        "required": ["word"]
                    }
                }
            ]
        })
    }

    /// `tools/call` 分派。
    pub fn call(&self, params: &Value) -> Result<Value, Error> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Tool("tools/call 缺少 name".into()))?;
        let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
        let result = match name {
            "lookup" => self.lookup(&arguments)?,
            "gloss" => self.gloss(&arguments)?,
            _ => return Err(Error::Tool(format!("不认识的工具 {name}"))),
        };
        let text = serde_json::to_string(&result)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    /// 拼音 → 候选。与 CLI `display::show` 同链路：喂入 → 查 → 补译文。
    fn lookup(&self, args: &Value) -> Result<Value, Error> {
        let input = args
            .get("input")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Tool("lookup 需要 input（拼音串）".into()))?;
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(10)
            .min(50) as usize;
        let mut engine = self.engine.borrow_mut();
        if let Some(fuzzy) = args.get("fuzzy").and_then(Value::as_str) {
            let mut rules = FuzzyRules::default();
            for name in fuzzy.split(',') {
                if name == "all" {
                    rules = FuzzyRules::ALL;
                } else if !rules.enable(name) {
                    return Err(Error::Tool(format!("不认识的模糊音规则 {name}")));
                }
            }
            engine.set_fuzzy(rules);
        }
        engine.clear();
        engine.set_input(input);
        let mut query = engine
            .query()
            .map_err(|e| Error::Tool(format!("查询失败: {e}")))?;
        engine.annotate(&mut query.candidates);
        Ok(query_json(&query, limit))
    }

    /// 词 → 译文：走 `annotate` 的单候选路径，与输入法候选窗的释义同一来源。
    fn gloss(&self, args: &Value) -> Result<Value, Error> {
        let word = args
            .get("word")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Tool("gloss 需要 word（要查的词）".into()))?;
        let mut list = CandidateList {
            items: vec![Candidate {
                text: word.to_owned(),
                kind: CandidateKind::Chinese,
                syllables: Vec::new(),
                reading: None,
                translation: None,
                aux_code: None,
            }],
        };
        self.engine.borrow().annotate(&mut list);
        let translation = list.items.first().and_then(|c| c.translation.clone());
        Ok(json!({ "word": word, "translation": translation }))
    }
}

/// `lookup` 的结果 JSON。`translation` 缺省 null（释义表没查到）。
fn query_json(query: &Query, limit: usize) -> Value {
    let segmentations: Vec<String> = query
        .segmentations
        .iter()
        .map(ToString::to_string)
        .collect();
    let candidates: Vec<Value> = query
        .candidates
        .items
        .iter()
        .take(limit)
        .map(|c| {
            json!({
                "text": c.text,
                "kind": kind_name(c.kind),
                "syllables": c.syllables,
                "translation": c.translation,
            })
        })
        .collect();
    json!({
        "input": query.text,
        "segmentations": segmentations,
        "correction": query.correction.as_ref().map(|c| json!({
            "original": c.original,
            "segmentation": c.segmentation.to_string(),
        })),
        "tail": query.tail,
        "candidates": candidates,
    })
}

fn kind_name(kind: CandidateKind) -> &'static str {
    match kind {
        CandidateKind::Chinese => "chinese",
        CandidateKind::Code => "code",
        CandidateKind::English => "english",
        CandidateKind::Cloud => "cloud",
        CandidateKind::Shortcut => "shortcut",
        CandidateKind::Custom(_) => "custom",
        CandidateKind::Emoji => "emoji",
        CandidateKind::Sentence => "sentence",
    }
}
