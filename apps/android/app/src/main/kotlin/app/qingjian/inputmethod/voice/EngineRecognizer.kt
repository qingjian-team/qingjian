package app.qingjian.inputmethod.voice

import android.annotation.SuppressLint
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.os.Handler
import android.os.Looper
import app.qingjian.inputmethod.core.Engine
import kotlin.concurrent.thread
import kotlin.math.log10
import kotlin.math.sqrt

/**
 * Rust 侧的识别方（云端、离线模型，Core 的 `Transcriber`）：这边用 [AudioRecord] 录单声道 16 kHz，
 * 一截截交给引擎，每 [POLL_MS] 取一次结果。「语音识别」没配好时 [available] 为 false，[VoiceInput] 不会选它。
 */
class EngineRecognizer(private val engine: Engine) : VoiceRecognizer {
    private val main = Handler(Looper.getMainLooper())
    private var listener: VoiceRecognizer.Listener? = null

    /** 录音线程看它停；主线程写。 */
    @Volatile
    private var recording = false

    override val available: Boolean get() = engine.voiceAvailable

    override val needsMicrophone = true

    // 入口先查过录音权限（VoiceInput），没有权限不会走到这里
    @SuppressLint("MissingPermission")
    override fun start(listener: VoiceRecognizer.Listener) {
        this.listener = listener
        engine.voiceStart(SAMPLE_RATE)
        recording = true
        thread(name = "qingjian-voice-record") {
            val size = maxOf(AudioRecord.getMinBufferSize(SAMPLE_RATE, CHANNEL, ENCODING), CHUNK * 2)
            val record = runCatching {
                AudioRecord(MediaRecorder.AudioSource.VOICE_RECOGNITION, SAMPLE_RATE, CHANNEL, ENCODING, size)
            }.getOrNull()
            if (record?.state != AudioRecord.STATE_INITIALIZED) {
                record?.release()
                main.post { fail("麦克风打不开") }
                return@thread
            }
            record.startRecording()
            val buffer = ShortArray(CHUNK)
            while (recording) {
                val read = record.read(buffer, 0, buffer.size)
                if (read < 0) break
                if (read == 0) continue
                val chunk = buffer.copyOf(read)
                val levels = levels(chunk)
                // 句柄只在主线程用；说完之后晚到的那截不再交
                main.post {
                    if (!recording) return@post
                    engine.voiceFeed(chunk)
                    levels.forEach { this.listener?.onLevel(it) }
                }
            }
            record.stop()
            record.release()
        }
        main.postDelayed(poll, POLL_MS)
    }

    override fun finish() {
        if (!recording) return
        recording = false
        engine.voiceFinish()
    }

    override fun cancel() {
        engine.voiceCancel()
        stop()
    }

    private val poll: Runnable = Runnable {
        val listener = listener ?: return@Runnable
        engine.pollVoice()?.let { (kind, text) ->
            when (kind) {
                "partial" -> listener.onPartial(text)
                "final" -> {
                    stop()
                    listener.onFinal(text)
                }
                else -> {
                    stop()
                    listener.onError(text)
                }
            }
        }
        if (this.listener != null) main.postDelayed(poll, POLL_MS)
    }

    private fun fail(message: String) {
        val listener = listener ?: return
        engine.voiceCancel()
        stop()
        listener.onError(message)
    }

    /** 收尾：录音线程跟着停（出错、服务端先结束时也得停，不然麦克风一直开着）。 */
    private fun stop() {
        recording = false
        main.removeCallbacks(poll)
        listener = null
    }

    /**
     * 这一截每 [SLICE] 个采样一个响度（0 到 1），波形一截动几格。按分贝算：语音识别音源不带自动增益，正常说话的均方根只有满幅的百分之几，
     * 线性放大要么说话时几乎不动、要么大声就顶满；[QUIET_DB] 到 [LOUD_DB] 之间线性铺开。
     */
    private fun levels(samples: ShortArray): List<Float> = (samples.indices step SLICE).map { start ->
        val end = minOf(start + SLICE, samples.size)
        var sum = 0.0
        for (i in start until end) sum += samples[i].toDouble() * samples[i]
        val rms = sqrt(sum / (end - start)) / Short.MAX_VALUE
        val db = if (rms > 0) 20 * log10(rms) else QUIET_DB
        ((db - QUIET_DB) / (LOUD_DB - QUIET_DB)).toFloat().coerceIn(0f, 1f)
    }

    private companion object {
        const val SAMPLE_RATE = 16_000
        const val CHANNEL = AudioFormat.CHANNEL_IN_MONO
        const val ENCODING = AudioFormat.ENCODING_PCM_16BIT

        /** 一截 100 ms。 */
        const val CHUNK = SAMPLE_RATE / 10
        const val POLL_MS = 100L
        /** 20 ms 一个响度。 */
        const val SLICE = SAMPLE_RATE / 50

        /** 低于它算没声（环境底噪大约在这一带），高于 [LOUD_DB] 顶满。 */
        const val QUIET_DB = -55.0
        const val LOUD_DB = -15.0
    }
}
