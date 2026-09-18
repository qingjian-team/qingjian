//! 青简 addon 工厂。
#include "qingjian.h"
namespace fcitx {
class QingjianFactory final : public AddonFactory {
public:
    AddonInstance *create(AddonManager *manager) override { return new QingjianEngine(manager); }
};
}
FCITX_ADDON_FACTORY_V2(qingjian, fcitx::QingjianFactory)
