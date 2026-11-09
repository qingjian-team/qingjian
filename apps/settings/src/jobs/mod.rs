//! 放在后台线程跑的事：测试云服务连接、检查更新、导入词库、下载模型与词库、取可下载的词库清单、升级后补下词库。完成后叫醒窗口，在 `App::wake` 里取结果。

pub mod cloud_test;
pub mod dict_index;
pub mod download;
pub mod import;
pub mod restore;
pub mod update_check;
