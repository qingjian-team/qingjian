//! [`Command`]：叫主动同步的线程做的事。

pub(super) enum Command {
    SyncAll,

    SyncPeer(String),
}
