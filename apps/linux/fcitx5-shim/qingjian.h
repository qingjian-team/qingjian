// C ABI 边界：与 apps/linux/host/src/lib.rs 的 #[unsafe(no_mangle)] 出口一一对应。
// 两边不同步会在链接期炸，不会静默错——改一边必改另一边。
//
// 字符串约定：返回的 const char* 指向 Rust 侧 thread_local 缓冲，
// 只在下一次 qj_ 调用前有效，拿到后必须当场拷进 std::string。
#pragma once

#include <cstdint>

extern "C" {

const char *qj_version();

// 装配引擎（读 ~/.local/share/qingjian 下的数据）；失败时 qj_init_error 给人话。
bool qj_init();
const char *qj_init_error();

// 按键：true = 吞掉，随后取 qj_take_commit / qj_preedit / 候选状态刷面板。
bool qj_key_event(uint32_t keyval, uint32_t state, bool release);

const char *qj_take_commit(); // NULL = 本次无上屏
const char *qj_preedit();     // "" = 没在组句（收窗）
int qj_preedit_cursor();      // 字节偏移

int qj_candidate_count();               // 当前页候选数
const char *qj_candidate_text(int i);   // 页内第 i 格文本
const char *qj_candidate_comment(int i); // 页内第 i 格译文（"" = 无）
int qj_highlight();                     // 页内高亮下标，-1 = 无
const char *qj_page_indicator();        // "1/28"（"" = 只有一页）
void qj_select(int i);                  // 鼠标点选页内第 i 格

// 本地整句模型定时驱动（每 20ms 调一次）：bit0 = 要重画面板，bit1 = 继续定时。
uint32_t qj_model_poll();

void qj_set_program(const char *program); // 当前应用名（变了才调）：按应用关英文候选
uint32_t qj_preedit_display();            // 拼音行位置：0 行内+窗口 / 1 只行内 / 2 只窗口

void qj_set_private(bool p); // 密码框：学习/日志静音（变了才调）
void qj_focus_in();
void qj_focus_out();
void qj_reset();

} // extern "C"
