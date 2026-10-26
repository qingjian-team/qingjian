package app.qingjian.inputmethod.cloud

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * 云端密钥的存取：Android Keystore 里生成一把 AES 钥匙（在安全芯片 / TEE 里，导不出来、只认青简这个应用，卸载即毁），
 * 密钥用它 AES-GCM 加密后只存密文（`cloud` 那份 prefs 的 [KEY_CIPHER]，Base64 的 IV + 密文）。
 * 解出来的明文只在内存里用，不写日志、不显示（设置页只露末尾几位）。应用本就关了系统备份（`allowBackup=false`）。
 */
object SecretStore {
    fun save(context: Context, secret: String) {
        prefs(context).edit().putString(KEY_CIPHER, seal(secret)).apply()
    }

    /** 没存过、或解不开（钥匙没了：清过应用数据之类）返回 null。 */
    fun load(context: Context): String? = prefs(context).getString(KEY_CIPHER, null)?.let(::open)

    /** 加密成可以存进 prefs 的一串（Base64 的 IV + 密文）；别的密钥（语音识别那份）也用它，同一把钥匙。 */
    fun seal(secret: String): String {
        val cipher = Cipher.getInstance(TRANSFORMATION).apply { init(Cipher.ENCRYPT_MODE, key()) }
        val sealed = cipher.iv + cipher.doFinal(secret.toByteArray(Charsets.UTF_8))
        return Base64.encodeToString(sealed, Base64.NO_WRAP)
    }

    /** [seal] 的反面；解不开返回 null。 */
    fun open(stored: String): String? {
        return runCatching {
            val sealed = Base64.decode(stored, Base64.NO_WRAP)
            val iv = sealed.copyOfRange(0, IV_BYTES)
            val cipher = Cipher.getInstance(TRANSFORMATION).apply {
                init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(TAG_BITS, iv))
            }
            String(cipher.doFinal(sealed, IV_BYTES, sealed.size - IV_BYTES), Charsets.UTF_8)
        }.getOrNull()
    }

    fun clear(context: Context) {
        prefs(context).edit().remove(KEY_CIPHER).apply()
    }

    fun prefs(context: Context) = context.getSharedPreferences("cloud", Context.MODE_PRIVATE)

    /** 取 Keystore 里那把钥匙，没有就生成（只能加解密，不能导出）。 */
    private fun key(): SecretKey {
        val store = KeyStore.getInstance(KEYSTORE).apply { load(null) }
        (store.getKey(ALIAS, null) as? SecretKey)?.let { return it }
        val spec = KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(256)
            .build()
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE).run {
            init(spec)
            generateKey()
        }
    }

    private const val KEYSTORE = "AndroidKeyStore"
    private const val ALIAS = "qingjian-cloud"
    private const val TRANSFORMATION = "AES/GCM/NoPadding"
    private const val IV_BYTES = 12
    private const val TAG_BITS = 128
    private const val KEY_CIPHER = "api_key_cipher"
}
