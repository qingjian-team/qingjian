// fcitx5 shim：唯一的 C++ 文件，只做转发，不放任何业务判断（architecture.md 约束一）。
// 引擎逻辑全在 Rust（qingjian.h 声明的 C ABI，实现在 apps/linux/host）。
#include "qingjian.h"

#include <memory>
#include <vector>
#include <string>

#include <fcitx/addonfactory.h>
#include <fcitx/addoninstance.h>
#include <fcitx/addonmanager.h>
#include <fcitx/candidatelist.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/inputpanel.h>
#include <fcitx/instance.h>
#include <fcitx-utils/event.h>
#include <fcitx-utils/log.h>
#include <fcitx-utils/trackableobject.h>

// 本地整句模型的轮询间隔（微秒）：模型一次二三十毫秒，20ms 一问。
constexpr uint64_t kModelPollUsec = 20000;

namespace {

// 页内某格的候选：文本 + 译文 comment；点选转回 Rust。
class QjCandidate final : public fcitx::CandidateWord {
public:
    QjCandidate(int offset, const std::string &text, const std::string &comment)
        : offset_(offset) {
        setText(fcitx::Text(text));
        if (!comment.empty()) {
            setComment(fcitx::Text(comment));
        }
    }

    void select(fcitx::InputContext *ic) const override;

private:
    int offset_;
};

class QingjianEngine final : public fcitx::InputMethodEngineV2 {
public:
    explicit QingjianEngine(fcitx::Instance *instance) : instance_(instance) {
        if (qj_init()) {
            ready_ = true;
            FCITX_INFO() << "青简就绪: " << qj_version();
        } else {
            FCITX_ERROR() << "青简引擎装配失败: " << qj_init_error();
        }
    }

    void keyEvent(const fcitx::InputMethodEntry & /*entry*/,
                  fcitx::KeyEvent &event) override {
        if (!ready_) {
            return;
        }
        // 密码框/敏感输入：与 macOS Secure Input 同语义，学习与日志静音。
        const bool isPrivate = event.inputContext()->capabilityFlags().testAny(
            fcitx::CapabilityFlags{fcitx::CapabilityFlag::Password,
                                   fcitx::CapabilityFlag::Sensitive});
        if (isPrivate != lastPrivate_) {
            qj_set_private(isPrivate);
            lastPrivate_ = isPrivate;
        }
        // 当前应用（program 名）：按应用关英文候选用，变了才跨 FFI。
        const std::string &program = event.inputContext()->program();
        if (program != lastProgram_) {
            qj_set_program(program.c_str());
            lastProgram_ = program;
        }
        const bool consumed =
            qj_key_event(event.rawKey().sym(), event.rawKey().states(),
                         event.isRelease());
        if (consumed) {
            event.filterAndAccept();
        }
        // 未吞掉的键也要同步：比如「组句中敲半角标点」= 候选先上屏、字符再透传，
        // 上屏文本必须赶在放行的按键之前发给应用。
        sync(event.inputContext());
    }

    void activate(const fcitx::InputMethodEntry & /*entry*/,
                  fcitx::InputContextEvent & /*event*/) override {
        qj_focus_in();
    }

    void deactivate(const fcitx::InputMethodEntry & /*entry*/,
                    fcitx::InputContextEvent &event) override {
        qj_focus_out();
        clearPanel(event.inputContext());
    }

    void reset(const fcitx::InputMethodEntry & /*entry*/,
               fcitx::InputContextEvent &event) override {
        qj_reset();
        clearPanel(event.inputContext());
    }

    // 每个键之后：上屏文本、重建 preedit 与候选面板。
    void sync(fcitx::InputContext *ic) {
        if (const char *commit = qj_take_commit()) {
            ic->commitString(commit);
        }
        std::string preedit = qj_preedit();
        // 非组句且面板本就空：跳过无谓的 reset + UI 刷新（普通打字每键都会走到这里）。
        if (preedit.empty() && !panelShown_) {
            return;
        }
        auto &panel = ic->inputPanel();
        panel.reset();
        panelShown_ = !preedit.empty();
        if (!preedit.empty()) {
            const int cursor = qj_preedit_cursor();
            fcitx::Text preeditText(preedit, fcitx::TextFormatFlag::Underline);
            preeditText.setCursor(cursor);
            // 拼音行位置按配置([general] preedit)：行内要应用支持，不支持就退候选窗口。
            const uint32_t display = qj_preedit_display();
            const bool inlineOk =
                ic->capabilityFlags().test(fcitx::CapabilityFlag::Preedit);
            if (display != 2 && inlineOk) {
                panel.setClientPreedit(preeditText);
            }
            if (display == 2 || display == 0 || !inlineOk) {
                panel.setPreedit(preeditText);
            }
            auto list = std::make_unique<fcitx::CommonCandidateList>();
            const int count = qj_candidate_count();
            list->setPageSize(count > 0 ? count : 1); // 分页在 Rust 侧，这里永远单页
            // 序号标签要自己设，CommonCandidateList 默认为空（主题只管画不管产）。
            std::vector<std::string> labels;
            labels.reserve(count);
            for (int i = 0; i < count; ++i) {
                labels.push_back(std::to_string(i + 1) + " ");
            }
            list->setLabels(labels);
            for (int i = 0; i < count; ++i) {
                std::string text = qj_candidate_text(i);
                std::string comment = qj_candidate_comment(i);
                list->append(std::make_unique<QjCandidate>(i, text, comment));
            }
            const int highlight = qj_highlight();
            if (highlight >= 0 && highlight < count) {
                list->setGlobalCursorIndex(highlight);
            }
            if (count > 0) {
                panel.setCandidateList(std::move(list));
            }
            // 页码 "1/28"：分页在 Rust 侧，fcitx5 只拿到当前页画不出总页数，
            // 用辅助行（候选下方）显示，对齐 macOS 候选窗右下角页码。
            std::string pageInfo = qj_page_indicator();
            if (!pageInfo.empty()) {
                panel.setAuxDown(fcitx::Text(pageInfo));
            }
        }
        ic->updatePreedit();
        ic->updateUserInterface(fcitx::UserInterfaceComponent::InputPanel);
        // 本地整句模型：每次重画后起（重起）轮询——按键、鼠标点选、定时重画都走到这里；
        // Rust 侧状态机没事等就不再续期。
        lastIc_ = ic->watch();
        armModelTimer();
    }

    void clearPanel(fcitx::InputContext *ic) {
        ic->inputPanel().reset();
        panelShown_ = false;
        ic->updatePreedit();
        ic->updateUserInterface(fcitx::UserInterfaceComponent::InputPanel);
    }

    // 本地整句模型的定时驱动：20ms 一问 Rust 侧状态机（防抖/请求/收分都在那边），
    // 要重画就重画，没事等了就不再续期，平时不占 CPU。
    void armModelTimer() {
        if (modelTimer_) {
            modelTimer_->setNextInterval(kModelPollUsec);
            modelTimer_->setOneShot();
            return;
        }
        modelTimer_ = instance_->eventLoop().addTimeEvent(
            CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + kModelPollUsec, 0,
            [this](fcitx::EventSourceTime *source, uint64_t /*usec*/) {
                const uint32_t poll = qj_model_poll();
                if (poll & 1) {
                    if (auto *ic = lastIc_.get()) {
                        sync(ic);
                    }
                }
                if (poll & 2) {
                    source->setNextInterval(kModelPollUsec);
                    source->setOneShot();
                }
                return true;
            });
    }

private:
    fcitx::Instance *instance_;
    bool ready_ = false;
    bool lastPrivate_ = false;
    bool panelShown_ = false;
    std::string lastProgram_;
    std::unique_ptr<fcitx::EventSourceTime> modelTimer_;
    fcitx::TrackableObjectReference<fcitx::InputContext> lastIc_;
};

// 点选：全局拿引擎不方便，直接调 Rust 再让事件循环里的 sync 兜底——
// 点选后必须立刻刷新面板，这里通过 ic 自己重建（与 keyEvent 后的 sync 同逻辑）。
QingjianEngine *g_engine = nullptr;

void QjCandidate::select(fcitx::InputContext *ic) const {
    qj_select(offset_);
    if (g_engine != nullptr) {
        g_engine->sync(ic);
    }
}

class QingjianFactory final : public fcitx::AddonFactory {
public:
    fcitx::AddonInstance *create(fcitx::AddonManager *manager) override {
        auto *engine = new QingjianEngine(manager->instance());
        g_engine = engine;
        return engine;
    }
};

} // namespace

FCITX_ADDON_FACTORY_V2(qingjian, QingjianFactory);
