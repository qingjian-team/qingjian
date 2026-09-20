//! 同用户连接内复用独立会话，拒绝未协商输入，关闭通知不产生回包。
use super::{NEXT_SESSION, Request, dispatch, dispatch_json, session::Session};
use crate::protocol::{DisplayAcknowledged, DisplayIdentity, LINUX_UI_PROTOCOL, LinuxEvent};
use qingjian_platform::protocol::{
    ClientMessage, Frame, PROTOCOL_VERSION, SessionId, read_message, write_message,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::sync::atomic::Ordering;
use std::sync::mpsc::SyncSender;
use std::time::Duration;

pub(super) fn serve_connection(
    mut stream: UnixStream,
    sender: SyncSender<Request>,
    settings: Value,
) {
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let trusted = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut size,
        ) == 0
            && credentials.uid == libc::geteuid()
    };
    if !trusted {
        return;
    }
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    let mut sessions = HashMap::<SessionId, Session>::new();
    while let Ok(Some(value)) = read_message::<_, Value>(&mut stream) {
        let response = if let Some(request) = value.get("LinuxHello") {
            let Some(local) = request
                .get("session")
                .and_then(|v| v.as_u64())
                .map(SessionId)
            else {
                break;
            };
            let Some(context) = request
                .get("context")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty() && s.len() <= 64)
            else {
                break;
            };
            let Some(generation) = request
                .get("generation")
                .and_then(|v| v.as_u64())
                .filter(|v| *v > 0)
            else {
                break;
            };
            if request.get("version").and_then(|v| v.as_u64()) != Some(u64::from(LINUX_UI_PROTOCOL))
                || sessions.values().any(|s| {
                    s.identity
                        .as_ref()
                        .is_some_and(|i| i.context == context || i.generation != generation)
                })
            {
                break;
            }
            let Some(session) = sessions.get_mut(&local).filter(|s| s.identity.is_none()) else {
                break;
            };
            let identity = DisplayIdentity {
                generation,
                context: context.into(),
                revision: 0,
            };
            dispatch_json(
                &sender,
                json!({"DisplayReporting": {"session": session.global, "identity": identity}}),
            );
            session.identity = Some(identity);
            let mut reply = settings.clone();
            reply["session"] = json!(local);
            Some(json!({"LinuxHello": reply}))
        } else if let Some(request) = value.get("DisplayAcknowledged") {
            let Ok(mut ack) = serde_json::from_value::<DisplayAcknowledged>(request.clone()) else {
                break;
            };
            if ack.senses.len() > 128 {
                break;
            }
            // 已关闭或过期的展示回执是正常延迟通知，不破坏其他会话。
            if let Some(session) = sessions.get(&ack.session)
                && session.identity.as_ref().is_some_and(|i| {
                    i.context == ack.identity.context && i.generation == ack.identity.generation
                })
            {
                ack.session = session.global;
                dispatch_json(&sender, json!({"DisplayAcknowledged": ack}));
            }
            None
        } else if let Some(request) = value.get("LinuxEvent") {
            let Some(local) = request
                .get("session")
                .and_then(|v| v.as_u64())
                .map(SessionId)
            else {
                break;
            };
            let Ok(event) = serde_json::from_value::<LinuxEvent>(
                request.get("event").cloned().unwrap_or(Value::Null),
            ) else {
                break;
            };
            let Some(session) = sessions.get_mut(&local) else {
                // 旧回调仍有同步调用者，明确返回忽略结果，不能遗留未读回包。
                if write_message(&mut stream, &json!({"Ignored": {"session": local}})).is_err() {
                    break;
                }
                continue;
            };
            let capabilities = matches!(event, LinuxEvent::Capabilities(_));
            if session.identity.is_none() || (!session.capabilities && !capabilities) {
                break;
            }
            session.capabilities |= capabilities;
            dispatch_json(
                &sender,
                json!({"LinuxEvent": {"session": session.global, "event": event}}),
            )
            .map(|reply| localize(reply, local))
        } else {
            let Ok(message) = serde_json::from_value::<ClientMessage>(value) else {
                break;
            };
            match message {
                ClientMessage::OpenSession {
                    session: local,
                    app,
                    protocol,
                } => {
                    if protocol != PROTOCOL_VERSION || sessions.contains_key(&local) {
                        break;
                    }
                    let global = SessionId(NEXT_SESSION.fetch_add(1, Ordering::Relaxed));
                    dispatch(
                        &sender,
                        ClientMessage::OpenSession {
                            session: global,
                            app,
                            protocol,
                        },
                    );
                    sessions.insert(
                        local,
                        Session {
                            global,
                            identity: None,
                            capabilities: false,
                        },
                    );
                    Some(
                        json!({"Update": {"session": local, "frame": Frame::default(), "linux_ui": settings}}),
                    )
                }
                ClientMessage::CloseSession { session } => {
                    if let Some(session) = sessions.remove(&session) {
                        dispatch(
                            &sender,
                            ClientMessage::CloseSession {
                                session: session.global,
                            },
                        );
                    }
                    None
                }
                ClientMessage::Privacy { session, private } => {
                    let Some(session) = sessions.get_mut(&session).filter(|s| s.identity.is_some())
                    else {
                        break;
                    };
                    dispatch(
                        &sender,
                        ClientMessage::Privacy {
                            session: session.global,
                            private,
                        },
                    );
                    session.capabilities = true;
                    None
                }
                ClientMessage::Key { session, event } => {
                    let Some(mapped) = sessions
                        .get(&session)
                        .filter(|s| s.identity.is_some() && s.capabilities)
                    else {
                        break;
                    };
                    dispatch_json(
                        &sender,
                        json!({"Key": {"session": mapped.global, "event": event}}),
                    )
                    .map(|v| localize(v, session))
                }
                ClientMessage::Poll { session } | ClientMessage::Commit { session } => {
                    let Some(mapped) = sessions
                        .get(&session)
                        .filter(|s| s.identity.is_some() && s.capabilities)
                    else {
                        break;
                    };
                    let name = if matches!(message, ClientMessage::Poll { .. }) {
                        "Poll"
                    } else {
                        "Commit"
                    };
                    dispatch_json(&sender, json!({name: {"session": mapped.global}}))
                        .map(|v| localize(v, session))
                }
                _ => break,
            }
        };
        if let Some(response) = response
            && write_message(&mut stream, &response).is_err()
        {
            break;
        }
    }
    for (_, session) in sessions {
        dispatch(
            &sender,
            ClientMessage::CloseSession {
                session: session.global,
            },
        );
    }
}
fn localize(mut response: Value, local: SessionId) -> Value {
    if let Some(body) = response.as_object_mut().and_then(|o| o.values_mut().next()) {
        body["session"] = json!(local);
    }
    response
}
