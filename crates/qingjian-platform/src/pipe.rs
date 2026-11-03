//! Windows 命名管道的对端身份。
//!
//! `\\.\pipe\` 是全局命名空间：谁都能建同名实例，谁都能往上连。名字挡不住抢注（抢在 Server 前面建，
//! 真 Server 拿 `FILE_FLAG_FIRST_PIPE_INSTANCE` 就建不出来、直接退出），能挡的是**连上之后验身份**：
//! Server 验客户端是不是本用户，DLL 验对端是不是本用户下跑的那个 `qingjian-server.exe`。抢注因此
//! 只能让输入法用不了，换不到宿主的光标前后文与按键。
//!
//! 管道名带用户 SID 派生的摘要，解决的是另一件事：多人共用一台机器（快速切换用户、远程桌面多会话）时
//! 各人的 Server 以前抢同一个名字——后登录的那个起不来，或者更糟：两个用户共用同一份 Engine。
//!
//! 验身份只到「同一用户」为止，够用也不够用：同一用户下的恶意进程本来就是同一权限，拦不住也无所谓；
//! 要拦的是别的用户（含 SYSTEM / 提权进程）和同用户的冒名者，前者靠令牌用户 SID，后者靠映像路径。

use std::fs;
use std::io;
use std::os::windows::io::RawHandle;
use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_INSUFFICIENT_BUFFER, HANDLE, HLOCAL, LocalFree,
};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, GetSecurityInfo, SE_KERNEL_OBJECT,
};
use windows::Win32::Security::{
    GetTokenInformation, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, RevertToSelf,
    TOKEN_INFORMATION_CLASS, TOKEN_OWNER, TOKEN_QUERY, TOKEN_USER, TokenIsAppContainer, TokenOwner,
    TokenUser,
};
use windows::Win32::System::Pipes::{GetNamedPipeServerProcessId, ImpersonateNamedPipeClient};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThread, OpenProcess, OpenProcessToken, OpenThreadToken,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::core::{HRESULT, PWSTR};

/// 管道名的固定前缀。后面接用户摘要，一个用户一条。
const PREFIX: &str = r"\\.\pipe\qingjian-";

/// 管道 DACL：放行 Everyone / ALL APPLICATION PACKAGES / ALL RESTRICTED APPLICATION PACKAGES，
/// 完整性标 Low。
///
/// 权限是 `0x12019F`＝`FILE_GENERIC_READ | FILE_GENERIC_WRITE`：读写管道数据 + 同步。以前给的是 `GA`，
/// 里面含 `WRITE_DAC` / `WRITE_OWNER` / `DELETE`——任何本地进程都能改这条管道的 ACL 或者删实例，
/// 而客户端真正需要的只有读写。放行哪些主体没动：任务栏搜索、设置这类 AppContainer 进程里的 DLL
/// 靠 `AC` / `S-1-15-2-2` 才连得上（受限令牌的 DACL 检查要求两个 SID 都放行）。谁能连到这里只是第一道
/// 门，连上之后还要过 [`verify_client`]。
pub const SDDL: &str =
    "D:(A;;0x12019F;;;WD)(A;;0x12019F;;;AC)(A;;0x12019F;;;S-1-15-2-2)S:(ML;;NW;;;LW)";

/// 映像路径缓冲的上限。真路径几百字符就够，给到 32 KiB 只是不想让「对面报个天文数字」把循环卡死。
const MAX_IMAGE_PATH: u32 = 32 * 1024;

/// 客户端认出 Server 到哪一步。
///
/// AppContainer 里的 DLL 打不开用户桌面上的进程（跨包的访问检查过不去），只能验管道所有者，
/// 所以把「认出到哪一步」告诉调用方，好让它记进日志而不是静默降级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerCheck {
    /// 映像路径与令牌用户都对上了。
    Image,
    /// 只验到管道所有者：本用户，或提升令牌下的 `BUILTIN\Administrators`（见 `owner_is_ours`）。
    Owner,
}

/// 由用户 SID 派生管道名：`\\.\pipe\qingjian-<12 位十六进制摘要>`。
///
/// 摘要只是为了让名字短且不带冒号，不承担安全职责——管道名在会话里枚举得到，真正的闸门是
/// [`verify_client`] / [`verify_server`]。
pub fn name(user_sid: &str) -> String {
    format!("{PREFIX}{:012x}", digest(user_sid.as_bytes()))
}

/// 当前用户的管道名。取不到令牌的调用方自己决定怎么退：DLL 记日志、放行字母，Server 直接退出。
pub fn current_name() -> io::Result<String> {
    Ok(name(&current_user_sid()?))
}

/// 当前进程令牌里的用户 SID（形如 `S-1-5-21-…-1001`）。
pub fn current_user_sid() -> io::Result<String> {
    user_sid_of_process(unsafe { GetCurrentProcess() })
}

/// 验客户端是不是本用户：临时借用调用者的令牌读它的用户 SID，读完立刻退出来。
///
/// **调用前必须已经从这条管道里读到过数据**：`ImpersonateNamedPipeClient` 在那之前一律报
/// `ERROR_CANNOT_IMPERSONATE`（令牌要等第一次读才挂到管道上）。所以顺序只能是「收下第一帧 → 验身份 →
/// 再处理」，不能反过来。正常路径上 DLL 连上就发 `OpenSession`，这一刻不会等。
///
/// 返回 `Err` 时调用方应直接断开、别回话，把已经读到的那帧也扔掉。AppContainer / 受限令牌里
/// `TokenUser` 仍是登录用户，所以商店应用照过。
pub fn verify_client(pipe: RawHandle) -> io::Result<()> {
    let ours = current_user_sid()?;
    unsafe { ImpersonateNamedPipeClient(HANDLE(pipe)) }?;
    // 借来的身份必须在离开本函数前还掉：这条线程接着要服务别的请求，带着别人的令牌跑是灾难。
    // 声明顺序即释放顺序的反序，先把令牌句柄收掉再 `RevertToSelf`。
    let _impersonating = Impersonation;
    let mut token = HANDLE::default();
    unsafe { OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, true, &mut token) }?;
    let _token = Handle(token);
    let client = user_sid_of_token(token)?;
    authorize(&client, &ours, "客户端")
}

/// 验对端是不是「本用户下跑的 `expected_exe`」。
///
/// 先认进程：服务端进程令牌里的用户 SID 必须与本进程一致，映像路径必须就是 `expected_exe`。
/// 打不开服务端进程时只有 AppContainer 宿主退到「只验管道所有者」（见 [`PeerCheck`]），其余一律拒绝。
///
/// 所有者为什么不能只比用户 SID：提升的管理员令牌建内核对象时，所有者取自令牌的 `DefaultOwner`，
/// 那是 `BUILTIN\Administrators`——只比用户 SID 会把管理员机器上的合法 Server 全判成冒名者。
/// 所以退路那条走 `owner_is_ours`：用户 SID 或本进程令牌的默认所有者，认一个就行。
pub fn verify_server(pipe: RawHandle, expected_exe: &Path) -> io::Result<PeerCheck> {
    let ours = current_user_sid()?;
    let mut pid = 0u32;
    unsafe { GetNamedPipeServerProcessId(HANDLE(pipe), &mut pid) }?;
    let process = match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) } {
        Ok(process) => process,
        // 只有 AppContainer 里的 DLL 有资格退到「只验所有者」：跨包访问用户桌面进程会被拒，
        // 那是宿主形态决定的，不是对端可疑。别的场景打不开进程一律当可疑（失败关门）。
        Err(error)
            if error.code() == HRESULT::from_win32(ERROR_ACCESS_DENIED.0) && in_app_container() =>
        {
            let owner = owner_sid(HANDLE(pipe))?;
            if !owner_is_ours(&owner, &ours)? {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("管道所有者 {owner} 与本进程的用户 {ours} 不符"),
                ));
            }
            return Ok(PeerCheck::Owner);
        }
        Err(error) => return Err(io::Error::other(error)),
    };
    let _process = Handle(process);

    authorize(&user_sid_of_process(process)?, &ours, "Server 用户")?;
    let image = image_path(process)?;
    if same_file(&image, expected_exe) {
        Ok(PeerCheck::Image)
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "对端不是 {}（实际是 {}）",
                expected_exe.display(),
                image.display()
            ),
        ))
    }
}

/// 管道所有者算不算「我们自己」。
///
/// 用户 SID 只是其中一种答案：提升的管理员令牌建对象时所有者是 `BUILTIN\Administrators`
/// （令牌的 `DefaultOwner`），普通与 AppContainer 令牌才是用户 SID。两边都认，否则管理员机器上
/// 「所有者是本用户」这条会是恒假——CI 的 Windows runner 正好是提升的，第一版就是这么红的。
fn owner_is_ours(owner: &str, ours: &str) -> io::Result<bool> {
    if owner.eq_ignore_ascii_case(ours) {
        return Ok(true);
    }
    Ok(owner.eq_ignore_ascii_case(&default_owner_sid()?))
}

/// 本进程令牌的默认所有者（`TokenOwner`）。
fn default_owner_sid() -> io::Result<String> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }?;
    let _token = Handle(token);
    let buffer = token_info(token, TokenOwner)?;
    let owner = unsafe { buffer.as_ptr().cast::<TOKEN_OWNER>().read_unaligned() };
    sid_to_string(owner.Owner)
}

/// 对端必须是本用户。比字符串：两边的 SID 都出自 `ConvertSidToStringSidW`，写法一致。
fn authorize(peer: &str, ours: &str, who: &str) -> io::Result<()> {
    if peer.eq_ignore_ascii_case(ours) {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!("{who} {peer} 与本进程的用户 {ours} 不符"),
    ))
}

/// 关句柄的小工具：这里的早退路径多，手写 `CloseHandle` 迟早漏一个。
struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

/// `ImpersonateNamedPipeClient` 的守卫，析构时把线程身份还回去。
struct Impersonation;

impl Drop for Impersonation {
    fn drop(&mut self) {
        let _ = unsafe { RevertToSelf() };
    }
}

/// 本进程是不是跑在 AppContainer 里（商店应用、任务栏搜索这类宿主）。
fn in_app_container() -> bool {
    let mut token = HANDLE::default();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.is_err() {
        return false;
    }
    let _token = Handle(token);
    let mut flag = 0u32;
    let mut needed = 0u32;
    unsafe {
        GetTokenInformation(
            token,
            TokenIsAppContainer,
            Some((&raw mut flag).cast()),
            size_of::<u32>() as u32,
            &mut needed,
        )
    }
    .is_ok()
        && flag != 0
}

/// 进程令牌里的用户 SID。`GetCurrentProcess` 的伪句柄也能直接喂进来。
fn user_sid_of_process(process: HANDLE) -> io::Result<String> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }?;
    let _token = Handle(token);
    user_sid_of_token(token)
}

/// 令牌里的用户 SID。
fn user_sid_of_token(token: HANDLE) -> io::Result<String> {
    let buffer = token_info(token, TokenUser)?;
    // 缓冲区开头就是 `TOKEN_USER`，`User.Sid` 指向同一块内存里的 SID
    let user = unsafe { buffer.as_ptr().cast::<TOKEN_USER>().read_unaligned() };
    sid_to_string(user.User.Sid)
}

/// 两段式取令牌信息：先问长度（必定报缓冲区不够，不算失败），再取内容。
fn token_info(token: HANDLE, class: TOKEN_INFORMATION_CLASS) -> io::Result<Vec<u8>> {
    let mut needed = 0u32;
    unsafe {
        let _ = GetTokenInformation(token, class, None, 0, &mut needed);
    }
    if needed == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut buffer = vec![0u8; needed as usize];
    unsafe {
        GetTokenInformation(
            token,
            class,
            Some(buffer.as_mut_ptr().cast()),
            needed,
            &mut needed,
        )?;
    }
    Ok(buffer)
}

/// 管道对象所有者的 SID（缺省是建它的进程的用户）。
fn owner_sid(pipe: HANDLE) -> io::Result<String> {
    let mut owner = PSID::default();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
        GetSecurityInfo(
            pipe,
            SE_KERNEL_OBJECT,
            OWNER_SECURITY_INFORMATION,
            Some(&mut owner),
            None,
            None,
            None,
            Some(&mut descriptor),
        )
    }
    .ok()?;
    // 描述符是 LocalAlloc 出来的，连它里面的 SID 一起在这里释放
    let _descriptor = Local(descriptor);
    sid_to_string(owner)
}

/// `LocalAlloc` 出来的安全描述符，析构时 `LocalFree`。
struct Local(PSECURITY_DESCRIPTOR);

impl Drop for Local {
    fn drop(&mut self) {
        if !self.0.0.is_null() {
            unsafe { LocalFree(Some(HLOCAL(self.0.0))) };
        }
    }
}

/// SID → `S-1-5-21-…`。`ConvertSidToStringSidW` 给的是 `LocalAlloc` 的宽字符串，转完要还回去。
fn sid_to_string(sid: PSID) -> io::Result<String> {
    let mut wide = PWSTR::null();
    unsafe { ConvertSidToStringSidW(sid, &mut wide)? };
    let text = unsafe { wide.to_string() };
    unsafe { LocalFree(Some(HLOCAL(wide.0.cast()))) };
    text.map_err(io::Error::other)
}

/// 进程的映像路径。缓冲不够时按 API 回报的长度重来（它不回长度就翻倍）。
fn image_path(process: HANDLE) -> io::Result<PathBuf> {
    let mut size = 1024u32;
    loop {
        let mut buffer = vec![0u16; size as usize];
        let mut written = size;
        let queried = unsafe {
            QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut written,
            )
        };
        match queried {
            Ok(()) => {
                buffer.truncate(written as usize);
                return Ok(PathBuf::from(String::from_utf16_lossy(&buffer)));
            }
            Err(error)
                if error.code() == HRESULT::from_win32(ERROR_INSUFFICIENT_BUFFER.0)
                    && size < MAX_IMAGE_PATH =>
            {
                size = written.max(size * 2).min(MAX_IMAGE_PATH);
            }
            Err(error) => return Err(io::Error::other(error)),
        }
    }
}

/// 两个路径是不是同一个文件。
///
/// 规范化能把大小写、8.3 短名、目录联接都归一（两边的文件都在，Server 正在跑），归一不了才退回
/// 不区分大小写的文本比较——比如安装目录被改名而在跑的进程还攥着旧路径。
fn same_file(actual: &Path, expected: &Path) -> bool {
    if actual == expected {
        return true;
    }
    if let (Ok(left), Ok(right)) = (fs::canonicalize(actual), fs::canonicalize(expected)) {
        return left == right;
    }
    actual
        .as_os_str()
        .to_string_lossy()
        .eq_ignore_ascii_case(&expected.as_os_str().to_string_lossy())
}

/// FNV-1a 64。散布够用，不做安全用途（见 [`name`]）。
fn digest(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Write};
    use std::os::windows::io::{AsRawHandle, FromRawHandle};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::thread;
    use std::time::Duration;

    use windows::Win32::Foundation::{ERROR_PIPE_CONNECTED, HANDLE};
    use windows::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
    use windows::Win32::Storage::FileSystem::{FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX};
    use windows::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
        PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    };
    use windows::core::{HRESULT, HSTRING};

    use super::{
        PeerCheck, SDDL, current_user_sid, name, owner_is_ours, owner_sid, verify_client,
        verify_server,
    };

    #[test]
    fn pipe_name_stays_inside_the_pipe_namespace() {
        let user = "S-1-5-21-1111111111-2222222222-3333333333-1001";
        let pipe = name(user);
        // 钉死算法：`tools/poc/pipe_name.py` 用同一套算名字，改了这里那边就指错管道
        assert_eq!(pipe, r"\\.\pipe\qingjian-64158933063b05a2");
        assert_eq!(pipe.matches('\\').count(), 4, "{pipe}");
        assert!(pipe.len() < 64, "{pipe}");
        // 名字只是命名空间，不该把 SID 原样摊出来
        assert!(!pipe.contains("S-1-5"), "{pipe}");
        assert_eq!(pipe, name(user), "同一个 SID 必须给出同一个名字");
    }

    #[test]
    fn different_users_get_different_pipes() {
        let users = [
            "S-1-5-21-1111111111-2222222222-3333333333-1001",
            "S-1-5-21-1111111111-2222222222-3333333333-1002",
            "S-1-5-18",
        ];
        let names: HashSet<String> = users.iter().map(|user| name(user)).collect();
        assert_eq!(names.len(), users.len(), "不同 SID 不该撞名：{names:?}");
    }

    /// 本进程的一条线程当客户端连上来：`verify_client` 必须认出「本用户」。
    /// 走的是真的 `ImpersonateNamedPipeClient` + 令牌查询，不是桩。
    #[test]
    fn verifies_a_client_of_the_same_user() {
        let server = pipe_pair();
        verify_client(server.as_raw_handle()).expect("本用户的客户端应当通过");
        assert!(!current_user_sid().expect("读得到本进程的 SID").is_empty());
    }

    /// 对端就是本进程：期望路径给当前测试 exe，`verify_server` 应当走到「映像路径对上」。
    #[test]
    fn verifies_the_server_image_path() {
        let server = pipe_pair();
        let exe = std::env::current_exe().expect("测试进程自己的路径");
        assert_eq!(
            verify_server(server.as_raw_handle(), &exe).expect("本进程就是服务端"),
            PeerCheck::Image
        );
    }

    /// 自己建的管道，所有者必须被 [`owner_is_ours`] 认下来。
    ///
    /// 提升的管理员令牌（CI 的 windows runner 就是）建对象时所有者是 `BUILTIN\Administrators`，
    /// 只比用户 SID 的版本会在这里判成「不是本用户」——管理员机器上的输入法因此完全连不上。
    #[test]
    fn the_pipe_owner_is_recognised_on_an_elevated_token() {
        let server = pipe_pair();
        let ours = current_user_sid().expect("读得到本进程的 SID");
        let owner = owner_sid(HANDLE(server.as_raw_handle())).expect("读得到管道所有者");
        assert!(
            owner_is_ours(&owner, &ours).expect("读得到令牌的默认所有者"),
            "自己建的管道认不出所有者：{owner}（本进程用户 {ours}）"
        );
        // 别的身份照样要拒，门没被整条撬开
        assert!(
            !owner_is_ours("S-1-5-18", &ours).expect("读得到令牌的默认所有者"),
            "SYSTEM 不该被当成我们自己"
        );
    }

    /// 期望路径写成别的 exe（就是抢注者该有的样子）：必须拒。
    #[test]
    fn rejects_a_server_whose_image_is_somewhere_else() {
        let server = pipe_pair();
        let elsewhere = std::env::current_exe()
            .expect("测试进程自己的路径")
            .with_file_name("qingjian-server.exe");
        let error = verify_server(server.as_raw_handle(), &elsewhere).expect_err("路径不符要拒");
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(
            error.to_string().contains("qingjian-server.exe"),
            "错误里要带对不上的路径，{error}"
        );
    }

    /// 建一条管道，本进程的另一条线程当客户端连上来并发一帧，返回服务端句柄。
    ///
    /// 客户端写、服务端读这一步不能省：`ImpersonateNamedPipeClient` 要等管道上有数据被读过才能用。
    /// 客户端的句柄由那条线程攥着（睡到进程结束），它一放，服务端这份就断了。
    fn pipe_pair() -> File {
        let pipe = format!(
            r"\\.\pipe\qingjian-self-{}-{}",
            std::process::id(),
            unique()
        );
        let mut server = create(&pipe);
        let client_name = pipe.clone();
        thread::spawn(move || {
            let mut client = connect(&client_name);
            client.write_all(b"ping").expect("写不进去");
            thread::sleep(Duration::from_secs(60));
            drop(client);
        });
        // 客户端可能在建实例与这里之间就连上了，那也算成功
        if let Err(error) = unsafe { ConnectNamedPipe(HANDLE(server.as_raw_handle()), None) }
            && error.code() != HRESULT::from_win32(ERROR_PIPE_CONNECTED.0)
        {
            panic!("ConnectNamedPipe 失败：{error}");
        }
        let mut first = [0u8; 1];
        server.read_exact(&mut first).expect("读不到客户端发的字节");
        server
    }

    /// 测试并行跑，同一个进程里管道名不能撞。
    fn unique() -> u32 {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        NEXT.fetch_add(1, Ordering::Relaxed)
    }

    fn connect(name: &str) -> File {
        for _ in 0..200 {
            if let Ok(file) = OpenOptions::new().read(true).write(true).open(name) {
                return file;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("连不上自己建的管道 {name}");
    }

    fn create(name: &str) -> File {
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                &HSTRING::from(SDDL),
                SDDL_REVISION_1,
                &mut descriptor,
                None,
            )
        }
        .expect("SDDL 应当解析得了");
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: false.into(),
        };
        let handle = unsafe {
            CreateNamedPipeW(
                &HSTRING::from(name),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                4096,
                4096,
                0,
                Some(&raw const attributes),
            )
        };
        assert!(
            !handle.is_invalid(),
            "建管道失败：{}",
            std::io::Error::last_os_error()
        );
        unsafe { File::from_raw_handle(handle.0) }
    }
}
