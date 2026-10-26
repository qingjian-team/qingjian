package app.qingjian.inputmethod.settings.page

import android.view.Gravity
import android.view.View
import android.widget.ImageView
import android.widget.LinearLayout
import app.qingjian.inputmethod.R

/** 关于：品牌标、名字、版本号，以及「全部在本机」的说明与许可。 */
class AboutActivity : PageActivity() {
    override val title = R.string.page_about

    override fun content(): List<View> {
        val version = packageManager.getPackageInfo(packageName, 0).versionName.orEmpty()
        return listOf(
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER_HORIZONTAL
                setPadding(0, ui.dp(28), 0, ui.dp(28))
                addView(ImageView(context).apply { setImageResource(R.drawable.ic_brand) }, LinearLayout.LayoutParams(ui.dp(72), ui.dp(72)))
                addView(ui.text(getString(R.string.app_name), 22f, ui.colors.text, bold = true).apply {
                    gravity = Gravity.CENTER
                    setPadding(0, ui.dp(14), 0, 0)
                })
                addView(ui.text(getString(R.string.about_version, version), 13f, ui.colors.secondary).apply {
                    gravity = Gravity.CENTER
                    setPadding(0, ui.dp(4), 0, 0)
                })
            },
            ui.card(
                ui.row(R.string.about_local_title, R.string.about_local_note),
                ui.divider(),
                ui.row(R.string.about_license_title, R.string.about_license_note),
            ),
        )
    }
}
