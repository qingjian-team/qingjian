//! 放在后台线程跑的事：测试云服务连接、检查更新、导入词库。完成后叫醒窗口，在 `App::wake` 里取结果。

pub mod cloud_test;
pub mod import;
pub mod update_check;
