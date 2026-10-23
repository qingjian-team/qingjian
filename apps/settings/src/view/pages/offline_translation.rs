//! 离线选区翻译的下载与开关，和整句候选排序模型分开。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{
    Button, ProgressBar, RenderError, SettingRow, Ui, group, paragraph, section_title,
};

use crate::jobs::model_download::Stage;
use crate::message::Message;
use crate::view::Page;
use crate::view::parts::controls::toggle_info;

pub(super) fn card(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let download = &page.settings.translation_download;
    let mut out = vec![section_title(ui, theme, "离线句子翻译（中英双向）")?];
    out.push(paragraph(ui, theme, "只在主动触发选区翻译时运行；断网可用，无需 API 账号。译文出现后按 Enter 确认；兼容模式下按提示复制并自行粘贴。", None)?);
    if !matches!(
        page.settings.config.general.learning_language.as_str(),
        "en" | "off"
    ) {
        out.push(paragraph(ui, theme, "当前离线模型支持中英双向。将学习语言改为英语后，可把中文选区译成英语；英文选区仍可译回中文。", None)?);
    }
    let installed = crate::system::paths::user_dir()
        .is_some_and(|dir| qingjian_offline::installed(&dir.join(qingjian_offline::MODEL_DIR)));
    let mut rows = Vec::new();
    if let Some(stage) = download.stage() {
        match stage {
            Stage::Downloading { done, total } => {
                let label = format!(
                    "正在下载 {:.0} / {:.0} MB",
                    done as f64 / 1_000_000.0,
                    total as f64 / 1_000_000.0
                );
                let cancel = Button::new("取消").build(
                    ui,
                    theme,
                    "cancel-offline",
                    Message::CancelTranslationDownload,
                )?;
                rows.push(SettingRow::new(&label).control(cancel).build(ui, theme)?);
                let ratio = if total == 0 {
                    0.0
                } else {
                    done as f32 / total as f32
                };
                let bar = ProgressBar::new(ratio).build(ui, theme)?;
                rows.push(
                    SettingRow::new("下载进度")
                        .control(bar)
                        .stacked()
                        .build(ui, theme)?,
                );
            }
            Stage::Installing => rows.push(SettingRow::new("正在校验并安装…").build(ui, theme)?),
            Stage::Failed(error) => out.push(paragraph(ui, theme, &error, None)?),
            Stage::Done => {}
        }
    }
    if !download.busy() && !installed {
        let bytes: u64 = qingjian_offline::packages()
            .iter()
            .flat_map(|pack| &pack.files)
            .map(|asset| asset.bytes)
            .sum();
        let label = format!("下载模型（约 {:.0} MB）", bytes as f64 / 1_000_000.0);
        let button = Button::new(&label).build(
            ui,
            theme,
            "download-offline",
            Message::DownloadTranslation,
        )?;
        rows.push(
            SettingRow::new("首次使用需下载")
                .control(button)
                .build(ui, theme)?,
        );
    }
    rows.push(toggle_info(
        ui,
        page,
        "启用离线翻译",
        Some("开启后选区翻译在本机完成，不会自动改用在线服务"),
        ("offline_translation", "enabled"),
        page.settings.config.offline_translation.enabled,
        (!installed && !page.settings.config.offline_translation.enabled) || download.busy(),
    )?);
    out.push(group(ui, theme, &rows)?);
    out.push(paragraph(ui, theme, "模型：Helsinki-NLP OPUS-MT，Xenova 导出 ONNX。中译英：CC-BY-4.0；英译中：Apache-2.0。每次最多选取 500 个 UTF-16 单元。", None)?);
    if let Some(("offline-translation", error)) = &page.settings.notice {
        out.push(paragraph(ui, theme, error, None)?);
    }
    Ok(out)
}
