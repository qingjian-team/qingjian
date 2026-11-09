//! 已配对设备的连接状态：只看实际连接的结果，不看 mDNS。组播被路由器拦掉时 mDNS 看不见对方，同步却照样通。

mod state;
mod status;

pub use state::PeerState;
pub use status::PeerStatus;

/// 最近一次成功和失败的时刻（毫秒）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Link {
    pub last_ok: Option<i64>,

    pub last_failure: Option<i64>,
}

/// 连上过多久之内算已连接，要比后台对账的间隔长。
const FRESH_MILLIS: i64 = 150_000;

impl Link {
    pub fn status(&self, now: i64) -> PeerStatus {
        match (self.last_ok, self.last_failure) {
            (ok, Some(failure)) if ok.is_none_or(|ok| failure > ok) => PeerStatus::Unreachable,
            (Some(ok), _) if now - ok <= FRESH_MILLIS => PeerStatus::Connected,
            _ => PeerStatus::Connecting,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Link, PeerStatus};

    fn link(last_ok: Option<i64>, last_failure: Option<i64>) -> Link {
        Link {
            last_ok,
            last_failure,
        }
    }

    #[test]
    fn status_follows_the_latest_real_connection() {
        assert_eq!(link(None, None).status(1000), PeerStatus::Connecting);
        assert_eq!(link(Some(900), None).status(1000), PeerStatus::Connected);
        assert_eq!(link(None, Some(900)).status(1000), PeerStatus::Unreachable);
        assert_eq!(
            link(Some(800), Some(900)).status(1000),
            PeerStatus::Unreachable
        );
        assert_eq!(
            link(Some(900), Some(800)).status(1000),
            PeerStatus::Connected
        );
    }

    #[test]
    fn an_old_success_is_no_longer_connected() {
        assert_eq!(link(Some(0), None).status(200_000), PeerStatus::Connecting);
    }
}
