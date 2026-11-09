package app.qingjian.inputmethod.core.strip

import android.graphics.RectF

/** 候选条位图上的一块可点区域（位图像素坐标）；[sense] 不为 null 时是这个候选下面的第几条译文。 */
data class StripHit(val candidate: Int, val sense: Int?, val rect: RectF)
