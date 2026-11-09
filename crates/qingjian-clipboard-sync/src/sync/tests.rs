use std::collections::BTreeMap;
use std::net::{TcpListener, TcpStream};
use std::thread;

use super::{Replica, run_initiator, run_responder};
use crate::channel::{accept, connect, read_opening};
use crate::error::SyncError;
use crate::identity::Identity;
use crate::record::{Record, RecordKind, Stamp};

#[derive(Default)]
struct Memory(BTreeMap<String, Record>);

impl Memory {
    fn with(records: Vec<Record>) -> Self {
        Self(
            records
                .into_iter()
                .map(|record| (record.stamp.id.clone(), record))
                .collect(),
        )
    }
}

impl Replica for Memory {
    fn stamps(&self) -> Result<Vec<Stamp>, SyncError> {
        Ok(self.0.values().map(|record| record.stamp.clone()).collect())
    }

    fn record(&self, id: &str) -> Result<Option<Record>, SyncError> {
        Ok(self.0.get(id).cloned())
    }

    fn apply(&mut self, record: Record) -> Result<(), SyncError> {
        self.0.insert(record.stamp.id.clone(), record);
        Ok(())
    }
}

fn round(mut left: Memory, mut right: Memory) -> (Memory, Memory, usize, usize) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (one, two) = (Identity::generate().unwrap(), Identity::generate().unwrap());
    let responder = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        read_opening(&mut stream).unwrap();
        let mut channel = accept(stream, &two, |_| true).unwrap();
        let applied = run_responder(&mut channel, &mut right, &mut |_| {}).unwrap();
        (right, applied)
    });
    let stream = TcpStream::connect(address).unwrap();
    let mut channel = connect(stream, &one, |_| true).unwrap();
    let left_applied = run_initiator(&mut channel, &mut left, &mut |_| {}).unwrap();
    let (right, right_applied) = responder.join().unwrap();
    (left, right, left_applied, right_applied)
}

#[test]
fn each_side_gets_what_it_lacks() {
    let left = Memory::with(vec![Record::text("from win".into(), 1)]);
    let right = Memory::with(vec![Record::image(vec![1, 2, 3], "png".into(), 2)]);
    let (left, right, left_applied, right_applied) = round(left, right);
    assert_eq!((left_applied, right_applied), (1, 1));
    assert_eq!(left.0, right.0);
    assert_eq!(left.0.len(), 2);
}

#[test]
fn second_round_transfers_nothing() {
    let left = Memory::with(vec![Record::text("a".into(), 1)]);
    let (left, right, _, _) = round(left, Memory::default());
    let (_, _, left_applied, right_applied) = round(left, right);
    assert_eq!((left_applied, right_applied), (0, 0));
}

#[test]
fn newer_delete_wins_and_older_delete_loses() {
    let text = Record::text("secret".into(), 5);
    let newer = Record::tombstone(
        Stamp {
            at: 6,
            ..text.stamp.clone()
        },
        RecordKind::Text,
    );
    let (left, right, _, _) = round(Memory::with(vec![text.clone()]), Memory::with(vec![newer]));
    assert!(left.0[&text.stamp.id].stamp.deleted);
    assert!(left.0[&text.stamp.id].text.is_empty());
    assert!(right.0[&text.stamp.id].stamp.deleted);

    let older = Record::tombstone(
        Stamp {
            at: 4,
            ..text.stamp.clone()
        },
        RecordKind::Text,
    );
    let (left, right, _, _) = round(Memory::with(vec![text.clone()]), Memory::with(vec![older]));
    assert!(!left.0[&text.stamp.id].stamp.deleted);
    assert_eq!(right.0[&text.stamp.id].text, "secret");
}

#[test]
fn same_second_prefers_the_delete() {
    let text = Record::text("x".into(), 5);
    let delete = Record::tombstone(text.stamp.clone(), RecordKind::Text);
    let (left, right, _, _) = round(Memory::with(vec![text.clone()]), Memory::with(vec![delete]));
    assert!(left.0[&text.stamp.id].stamp.deleted);
    assert!(right.0[&text.stamp.id].stamp.deleted);
}

#[test]
fn forged_content_is_detected() {
    let mut record = Record::text("real".into(), 1);
    assert!(record.is_consistent());
    record.text = "forged".into();
    assert!(!record.is_consistent());
}

#[test]
fn forgetting_stays_local_and_does_not_bring_the_item_back() {
    let text = Record::text("old".into(), 5);
    let mut forgotten = Record::tombstone(
        Stamp {
            at: 6,
            ..text.stamp.clone()
        },
        RecordKind::Text,
    );
    forgotten.stamp.forgotten = true;
    let (left, right, left_applied, right_applied) = round(
        Memory::with(vec![text.clone()]),
        Memory::with(vec![forgotten]),
    );
    assert_eq!((left_applied, right_applied), (0, 0));
    assert!(!left.0[&text.stamp.id].stamp.deleted);
    assert!(right.0[&text.stamp.id].stamp.forgotten);
}
