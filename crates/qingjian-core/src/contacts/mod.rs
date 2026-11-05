//! 通讯录：壳从系统通讯录读出的联系人，名字转成一张小词库并进查词，上屏名字后能接着给出这个人的电话、邮箱。
//!
//! 只在本机用：名字与号码不发给云端、不进学习记录与输入日志（见 `docs/design/contacts.md`）。

mod book;
mod contact;
mod reading;
mod surname;

pub use book::ContactBook;
pub use contact::Contact;

#[cfg(test)]
mod tests;
