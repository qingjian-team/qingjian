//! 隔离物理与软键盘候选复用的位图和中转缓冲区，避免刷新时相互覆盖。

package app.qingjian.inputmethod.candidates.physical

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.PorterDuff
import android.graphics.PorterDuffXfermode
import android.graphics.Rect
import java.nio.ByteBuffer

internal class BitmapCopy {
    private var snapshotBitmap: Bitmap? = null

    private var copyBuffer: ByteBuffer? = null

    private val copyPaint = Paint().apply { xfermode = PorterDuffXfermode(PorterDuff.Mode.SRC) }

    fun copy(src: Bitmap): Bitmap? {
        if (src.isRecycled || src.width <= 0 || src.height <= 0) return null
        val w = src.width
        val h = src.height
        val bytes = w * h * 4
        val dst = snapshotBitmap?.takeIf {
            !it.isRecycled && it.config == Bitmap.Config.ARGB_8888 && it.allocationByteCount >= bytes
        }?.apply {
            if (width != w || height != h) reconfigure(w, h, Bitmap.Config.ARGB_8888)
        } ?: Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888).also { snapshotBitmap = it }

        if (src.config == Bitmap.Config.ARGB_8888 && src.rowBytes == w * 4 && src.byteCount == bytes) {
            val buf = copyBuffer?.takeIf { it.capacity() >= bytes }
                ?: ByteBuffer.allocateDirect(bytes).also { copyBuffer = it }
            buf.clear()
            buf.limit(bytes)
            src.copyPixelsToBuffer(buf)
            buf.position(0)
            dst.copyPixelsFromBuffer(buf)
            buf.clear()
        } else {
            dst.eraseColor(Color.TRANSPARENT)
            Canvas(dst).drawBitmap(src, null, Rect(0, 0, w, h), copyPaint)
        }
        return dst
    }
}
