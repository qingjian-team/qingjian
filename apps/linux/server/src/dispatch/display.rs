//! Linux 版本化显示初始化与实际呈现确认，Windows 协议保持不变。
use super::Router;
use crate::protocol::{
    DisplayAcknowledged, DisplayIdentity, LINUX_UI_PROTOCOL, LinuxEvent, LinuxRequest,
};
use qingjian_core::Translation;
use qingjian_platform::protocol::{ClientMessage, ServerMessage, SessionId};
use serde_json::{Value, json};

impl Router {
    pub fn display_settings(&self) -> Value {
        json!({"version": LINUX_UI_PROTOCOL, "preedit": self.config.preedit})
    }

    pub fn handle_linux(&mut self, value: Value) -> Option<Value> {
        if let Some(body) = value.get("DisplayReporting") {
            let session: SessionId = serde_json::from_value(body.get("session")?.clone()).ok()?;
            let identity: DisplayIdentity =
                serde_json::from_value(body.get("identity")?.clone()).ok()?;
            let info = self.sessions.get_mut(&session)?;
            info.display_identity = Some(identity);
            info.display_frame = None;
            if self.focused == Some(session) {
                self.engine.note_displayed(std::iter::empty());
            }
            return None;
        }
        if let Some(body) = value.get("DisplayAcknowledged") {
            let ack: DisplayAcknowledged = serde_json::from_value(body.clone()).ok()?;
            self.acknowledge_display(ack);
            return None;
        }
        let response = if let Some(body) = value.get("LinuxEvent") {
            let request: LinuxRequest = serde_json::from_value(body.clone()).ok()?;
            if let LinuxEvent::Candidate { identity, .. } | LinuxEvent::Page { identity, .. } =
                &request.event
                && !self.valid_panel_event(request.session, identity)
            {
                // 旧鼠标事件不能改变其他会话的焦点、显示身份或学习回报。
                return Some(json!({"Ignored": {"session": request.session}}));
            }
            self.linux_event(request)?
        } else {
            let message: ClientMessage = serde_json::from_value(value).ok()?;
            self.handle(message)?
        };
        let mut value = serde_json::to_value(&response).ok()?;
        match &response {
            ServerMessage::KeyResult { session, frame, .. }
            | ServerMessage::Update { session, frame } => {
                self.display_revision += 1;
                let info = self.sessions.get_mut(session)?;
                if let Some(identity) = &mut info.display_identity {
                    if self.focused == Some(*session) {
                        self.engine.note_displayed(std::iter::empty());
                    }
                    identity.revision = self.display_revision;
                    info.display_frame = (!info.private && info.active).then(|| frame.clone());
                    value.as_object_mut()?.values_mut().next()?["identity"] = json!(identity);
                }
            }
            ServerMessage::Committed { session, .. } => {
                self.display_revision += 1;
                if let Some(info) = self.sessions.get_mut(session) {
                    info.display_frame = None;
                    if let Some(identity) = &mut info.display_identity {
                        identity.revision = self.display_revision;
                    }
                }
            }
            _ => {}
        }
        Some(value)
    }

    fn acknowledge_display(&mut self, ack: DisplayAcknowledged) {
        if self.focused != Some(ack.session) || ack.senses.len() > 128 {
            return;
        }
        let Some(info) = self.sessions.get(&ack.session) else {
            return;
        };
        if info.private || !info.active || info.display_identity.as_ref() != Some(&ack.identity) {
            return;
        }
        let Some(frame) = &info.display_frame else {
            return;
        };
        // 所有索引先校验，坏回报不能产生半份记录。
        if ack.senses.iter().any(|(row, sense)| {
            frame
                .candidates
                .items
                .get(*row)
                .filter(|c| !c.text.is_empty())
                .and_then(|c| c.translation.as_ref())
                .is_none_or(|t| *sense >= t.senses().len())
        }) {
            return;
        }
        let candidates: Vec<_> = frame
            .candidates
            .items
            .iter()
            .enumerate()
            .filter_map(|(row, candidate)| {
                let translation = candidate.translation.as_ref()?;
                let senses = translation
                    .senses()
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| ack.senses.contains(&(row, *i)))
                    .map(|(_, s)| s.clone())
                    .collect::<Vec<_>>();
                if senses.is_empty() {
                    return None;
                }
                let mut candidate = candidate.clone();
                candidate.translation = Some(Translation::new(translation.language, senses));
                Some(candidate)
            })
            .collect();
        self.engine.note_displayed(candidates.iter());
    }
}
