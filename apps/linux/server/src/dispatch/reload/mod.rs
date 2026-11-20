//! 后台检查配置与资源变化；输入线程只领取已读好的结果并更新引擎。

mod apply;
mod cloud;
mod prepared;
mod state;
mod task;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant, SystemTime};

use qingjian_platform::{Config, UserDirs};

pub(crate) use self::state::ConfigReload;
pub(crate) use self::task::ReloadTask;
use crate::assembly;
use crate::dispatch::Router;

/// 完成一批扫描后，至少间隔一秒再启动下一批。
const CONFIG_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// 正在加载时只轮询结果通道，避免空闲输入法多等一个一秒节拍。
pub(super) const RESULT_POLL_INTERVAL: Duration = Duration::from_millis(20);

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

impl Router {
    /// 启动时装配方案、记录初始文件状态；之后的检查和读取全部交给后台任务。
    pub fn watch_config(
        &mut self,
        config: &Config,
        config_path: PathBuf,
        root: PathBuf,
        user: UserDirs,
    ) {
        self.reload_task = None;
        self.apply_schemes(config, &root, &user.custom());
        let mut reload = ConfigReload {
            applied_config: config.clone(),
            scheme_files: Vec::new(),
            last_mtime: mtime(&config_path),
            config_path,
            last_check: Instant::now(),
            root,
            user,
            dictionary_files: Vec::new(),
            glossary_files: Vec::new(),
            applied_dictionaries: config.dictionaries.clone(),
            applied_model: config.model.clone(),
            applied_predict: config.predict.clone(),
            applied_language: assembly::learning_language(config),
        };
        reload.dictionary_files = reload.dict_snapshot();
        reload.glossary_files = reload.glossary_snapshot();
        reload.scheme_files = reload.scheme_snapshot();
        self.reload = Some(reload);
    }

    /// 至多一批任务在执行；文件系统卡顿时保持旧配置并继续处理按键。
    pub(super) fn poll_config_reload(&mut self) {
        if let Some(task) = &self.reload_task {
            match task.poll() {
                Ok(prepared) => {
                    self.reload_task = None;
                    self.apply_reload(prepared);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.reload_task = None;
                    if let Some(reload) = &mut self.reload {
                        reload.last_check = Instant::now();
                    }
                    tracing::warn!(
                        "configuration reload worker stopped, keeping previous configuration"
                    );
                }
            }
            return;
        }
        let Some(reload) = &mut self.reload else {
            return;
        };
        if reload.last_check.elapsed() < CONFIG_POLL_INTERVAL {
            return;
        }
        reload.last_check = Instant::now();
        match ReloadTask::spawn(
            reload.clone(),
            self.scheme_slot.clone(),
            self.engine.code_scheme().is_some(),
        ) {
            Ok(task) => self.reload_task = Some(task),
            Err(error) => tracing::warn!(%error, "failed to start configuration reload worker"),
        }
    }
}
