//! 成对符号（#928）要 DLL 对文档做的事：上屏后光标退回、跳过补上的右半边。
//! 光标后是什么只有 DLL 读得到，所以跳过是「核对过才做，对不上就照常插入」。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaretEdit {
    /// 上屏之后光标往回挪这么多个字符，停在补上的一对中间。
    Back(u32),

    /// 上屏文本的最后一个字是这个右半边：光标后面正是它就不插这个字、光标跳过它；不是就照常插入。
    Skip(char),
}
