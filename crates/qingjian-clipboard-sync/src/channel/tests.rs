use std::net::{TcpListener, TcpStream};
use std::thread;

use super::{
    NOT_PAIRED, Opening, accept, connect, generate_pair_code, pair_host, pair_join, read_opening,
};
use crate::error::SyncError;
use crate::identity::Identity;

fn tcp_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).unwrap();
    let (server, _) = listener.accept().unwrap();
    (client, server)
}

#[test]
fn right_code_pairs_and_both_learn_the_other_key() {
    let (host, join) = (Identity::generate().unwrap(), Identity::generate().unwrap());
    let (host_public, join_public) = (host.public, join.public);
    let (client, mut server) = tcp_pair();
    let hosting = thread::spawn(move || {
        assert_eq!(read_opening(&mut server).unwrap(), Opening::Pair);
        let channel = pair_host(server, &host, "123456").unwrap();
        assert_eq!(channel.remote(), &join_public);
    });
    let channel = pair_join(client, &join, "123456").unwrap();
    assert_eq!(channel.remote(), &host_public);
    hosting.join().unwrap();
}

#[test]
fn wrong_code_fails_on_both_sides() {
    let (host, join) = (Identity::generate().unwrap(), Identity::generate().unwrap());
    let (client, mut server) = tcp_pair();
    let hosting = thread::spawn(move || {
        read_opening(&mut server).unwrap();
        pair_host(server, &host, "123456").is_err()
    });
    assert!(pair_join(client, &join, "654321").is_err());
    assert!(hosting.join().unwrap());
}

#[test]
fn unpaired_device_is_refused() {
    let (host, stranger) = (Identity::generate().unwrap(), Identity::generate().unwrap());
    let host_public = host.public;
    let (client, mut server) = tcp_pair();
    let hosting = thread::spawn(move || {
        read_opening(&mut server).unwrap();
        accept(server, &host, |_| false).err()
    });
    let _ = connect(client, &stranger, |key| key == &host_public);
    assert!(matches!(
        hosting.join().unwrap(),
        Some(SyncError::UnknownPeer)
    ));
}

#[test]
fn a_forgotten_peer_is_told_it_is_no_longer_paired() {
    let (host, phone) = (Identity::generate().unwrap(), Identity::generate().unwrap());
    let host_public = host.public;
    let (client, mut server) = tcp_pair();
    let hosting = thread::spawn(move || {
        read_opening(&mut server).unwrap();
        accept(server, &host, |_| false).err()
    });
    let mut channel = connect(client, &phone, |key| key == &host_public).unwrap();
    channel.send(b"[]").unwrap();
    assert_eq!(channel.receive().unwrap(), NOT_PAIRED);
    assert!(matches!(
        hosting.join().unwrap(),
        Some(SyncError::UnknownPeer)
    ));
}

#[test]
fn large_messages_survive_chunking_and_are_encrypted() {
    let (host, join) = (Identity::generate().unwrap(), Identity::generate().unwrap());
    let (client, mut server) = tcp_pair();
    let message: Vec<u8> = (0..300_000_u32).map(|index| index as u8).collect();
    let expected = message.clone();
    let hosting = thread::spawn(move || {
        read_opening(&mut server).unwrap();
        let mut channel = accept(server, &host, |_| true).unwrap();
        assert_eq!(channel.receive().unwrap(), expected);
        channel.send(b"").unwrap();
    });
    let mut channel = connect(client, &join, |_| true).unwrap();
    channel.send(&message).unwrap();
    assert!(channel.receive().unwrap().is_empty());
    hosting.join().unwrap();
}

#[test]
fn pair_codes_have_six_digits() {
    for _ in 0..50 {
        let code = generate_pair_code().unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.bytes().all(|byte| byte.is_ascii_digit()));
    }
}
