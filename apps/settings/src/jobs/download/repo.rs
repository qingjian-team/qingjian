//! 从哪个仓库下：模型仓库还是词库仓库。两个仓库在 Gitee 与 GitHub 上各一份，附件一模一样。

/// 设了它就只从这个地址下（本机起个静态文件服务测下载用）：Release 附件是 `<地址>/<标签>/<文件>`，仓库里的文件是 `<地址>/<路径>`。
const MIRROR_ENV: &str = "QINGJIAN_DOWNLOAD_MIRROR";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repo {
    Models,
    Dicts,
}

impl Repo {
    fn name(self) -> &'static str {
        match self {
            Self::Models => "qingjian-models",
            Self::Dicts => "qingjian-dicts",
        }
    }

    /// Release 附件的地址前缀，后面接 `/<标签>/<文件>`。大陆多半 Gitee 快、海外多半 GitHub 快，两边同时取清单，先回来的先用。
    pub fn release_hosts(self) -> Vec<String> {
        if let Some(mirror) = mirror() {
            return vec![mirror];
        }
        let name = self.name();
        vec![
            format!("https://gitee.com/qingjian-team/{name}/releases/download"),
            format!("https://github.com/qingjian-team/{name}/releases/download"),
        ]
    }

    /// 仓库 main 分支里文件的地址前缀，后面接 `/<路径>`。
    pub fn raw_hosts(self) -> Vec<String> {
        if let Some(mirror) = mirror() {
            return vec![mirror];
        }
        let name = self.name();
        vec![
            format!("https://gitee.com/qingjian-team/{name}/raw/main"),
            format!("https://raw.githubusercontent.com/qingjian-team/{name}/main"),
        ]
    }
}

fn mirror() -> Option<String> {
    std::env::var(MIRROR_ENV)
        .ok()
        .map(|value| value.trim_end_matches('/').to_owned())
        .filter(|value| !value.is_empty())
}
