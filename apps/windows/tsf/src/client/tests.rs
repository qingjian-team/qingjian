//! 确认备用选区的专用回包不会被客户端变成文档上屏或结束等待。

use std::io::{Cursor, Read, Write};

use qingjian_platform::protocol::{
    InputSettings, KeyEvent, KeyModifiers, ScreenRect, ServerMessage, SessionId, write_message,
};

use super::{EngineClient, KeyReply};

struct Responses(Cursor<Vec<u8>>);

impl Read for Responses {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(out)
    }
}

impl Write for Responses {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn dedicated_review_reply_keeps_confirmation_out_of_the_document() {
    let session = SessionId(1);
    let mut bytes = Vec::new();
    for message in [
        ServerMessage::SessionOpened {
            session,
            input: InputSettings::default(),
        },
        ServerMessage::SelectionReview {
            session,
            active: true,
        },
        ServerMessage::SelectionReview {
            session,
            active: true,
        },
        ServerMessage::SelectionReview {
            session,
            active: false,
        },
    ] {
        write_message(&mut bytes, &message).unwrap();
    }
    let (mut client, _) = EngineClient::open(Responses(Cursor::new(bytes)), session, None).unwrap();
    assert!(
        client
            .fallback_selection(
                3,
                42,
                99,
                String::new(),
                ScreenRect {
                    left: 0,
                    top: 0,
                    right: 1,
                    bottom: 1
                }
            )
            .unwrap()
    );
    for active in [true, false] {
        let KeyReply::Result(reply) = client
            .key(KeyEvent::new(13, None, KeyModifiers::default()))
            .unwrap()
        else {
            panic!("review reply")
        };
        assert_eq!(reply.reviewing_selection, active);
        assert!(reply.commit.is_none());
        assert!(reply.frame.preedit.is_empty());
    }
}
