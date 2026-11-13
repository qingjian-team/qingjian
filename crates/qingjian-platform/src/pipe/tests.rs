use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::path::Path;

use windows::Win32::Foundation::{HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, ConvertStringSidToSidW, SDDL_REVISION_1,
};
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES};
use windows::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX;
use windows::Win32::System::Pipes::{CreateNamedPipeW, PIPE_TYPE_BYTE, PIPE_WAIT};
use windows::core::HSTRING;

use super::{
    CLIENT_ACCESS, CLIENT_QOS, UserSid, client_is_same_user, pipe_name, pipe_sddl,
    server_is_trusted,
};

/// 按产品的安全描述符建一个管道实例，再按 DLL 的方式连上。
fn connected_pair(tag: &str) -> (File, File) {
    let me = UserSid::current().unwrap();
    let name = format!(r"\\.\pipe\qingjian-test-{tag}-{}", std::process::id());
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            &HSTRING::from(pipe_sddl(&me).unwrap()),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
    }
    .unwrap();
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: false.into(),
    };
    let handle = unsafe {
        CreateNamedPipeW(
            &HSTRING::from(name.as_str()),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_WAIT,
            1,
            4096,
            4096,
            0,
            Some(&raw const attributes),
        )
    };
    assert!(!handle.is_invalid(), "建管道失败");
    let server = unsafe { File::from_raw_handle(handle.0) };
    let client = OpenOptions::new()
        .access_mode(CLIENT_ACCESS)
        .security_qos_flags(CLIENT_QOS)
        .open(&name)
        .expect("按 DLL 的权限打开管道");
    (server, client)
}

fn handle(file: &File) -> HANDLE {
    HANDLE(file.as_raw_handle())
}

fn sid(text: &str) -> UserSid {
    let mut raw = PSID::default();
    unsafe { ConvertStringSidToSidW(&HSTRING::from(text), &mut raw) }.unwrap();
    let sid = UserSid::copy(raw).unwrap();
    let _ = unsafe { LocalFree(Some(HLOCAL(raw.0))) };
    sid
}

#[test]
fn pipe_name_carries_the_user_and_the_logon_session() {
    let me = UserSid::current().unwrap();
    let name = pipe_name(&me).unwrap();
    let prefix = format!(r"\\.\pipe\qingjian-{}-", me.to_sddl().unwrap());
    assert!(name.starts_with(&prefix), "{name}");
    assert!(name[prefix.len()..].parse::<u32>().is_ok(), "{name}");
}

#[test]
fn server_recognizes_the_same_user() {
    let me = UserSid::current().unwrap();
    let (mut server, mut client) = connected_pair("client");
    client.write_all(b"x").unwrap();
    let mut byte = [0u8; 1];
    server.read_exact(&mut byte).unwrap();
    assert!(client_is_same_user(handle(&server), &me));
    assert!(!client_is_same_user(handle(&server), &sid("S-1-5-18")));
}

#[test]
fn client_trusts_only_the_expected_server_image_and_owner() {
    let me = UserSid::current().unwrap();
    let (_server, client) = connected_pair("server");
    let this_exe = std::env::current_exe().unwrap();
    assert!(server_is_trusted(handle(&client), &this_exe, &me));
    assert!(
        !server_is_trusted(
            handle(&client),
            Path::new(r"C:\Program Files\Qingjian\qingjian-server.exe"),
            &me
        ),
        "映像不对必须拒"
    );
    assert!(
        !server_is_trusted(handle(&client), &this_exe, &sid("S-1-5-18")),
        "所有者不是本用户必须拒"
    );
}
