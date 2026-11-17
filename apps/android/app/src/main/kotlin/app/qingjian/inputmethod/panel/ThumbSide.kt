package app.qingjian.inputmethod.panel

/**
 * 常用手是哪只 —— 面板里「顺手的那一侧」据此摆放。
 *
 * 与 `Settings.Arrangement` 一一对应，但面板只关心左右，不必知道键盘那几套排法叫什么。
 * 面板在 `panel` 包里而不在 `settings` 里：面板是画与点的地方，不该反过来依赖设置对象。
 */
enum class ThumbSide {
    /** 原来的排法：不按哪只手挪。 */
    NONE,

    /** 左手持机：分类列与返回放左边。 */
    LEFT,

    /** 右手持机：分类列与返回放右边。 */
    RIGHT,
}
