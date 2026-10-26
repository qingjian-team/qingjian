//! Engine 装配与候选条绘制的错误。
use qingjian_dictionary::DictionaryError;
use qingjian_render::RenderError;

/// 装配 Engine 时的错误；除主词库外的数据缺了只少功能，不算错。候选条画不出来时壳退回空白条。
#[derive(Debug, thiserror::Error)]
pub enum AndroidError {
    #[error("load dictionary: {0}")]
    Dictionary(#[from] DictionaryError),

    #[error("render candidate strip: {0}")]
    Render(#[from] RenderError),
}
