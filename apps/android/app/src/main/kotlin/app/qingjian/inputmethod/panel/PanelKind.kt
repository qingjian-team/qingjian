package app.qingjian.inputmethod.panel

/**
 * 盖住候选条与键盘的几种面板：logo 进功能面板，键盘图标进键盘选择，「符」进分类符号，笑脸进表情，
 * 候选条右端的 ⌄ 进候选展开；剪贴板图标进剪贴板（与常用语同一个面板，[PHRASES] 是停在常用语页），剪贴板里点「拆词」进 [PIECES]；
 * 说话时盖上 [VOICE]（[VoicePanel]，由语音输入开关，不经 [PanelActions.openPanel]）。
 */
enum class PanelKind { FUNCTIONS, KEYBOARDS, SYMBOLS, EMOJI, CANDIDATES, CLIPBOARD, PHRASES, PIECES, VOICE }
