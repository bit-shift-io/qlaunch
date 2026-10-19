package bitshift.qlaunch

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.net.Uri
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.drawable.BitmapDrawable
import android.graphics.drawable.Drawable
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream

/// Everything the Rust core asks the OS for. Rust owns all data and logic; this
/// class only answers questions and performs OS actions. Methods are `@JvmStatic`
/// so Rust finds them as static methods on `bitshift/qlaunch/QLaunchBridge`.
object QLaunchBridge {
    private var appContext: Context? = null

    @JvmStatic
    fun install(context: Context) {
        appContext = context.applicationContext
    }

    @JvmStatic
    fun getAppContext(): Context =
        appContext ?: throw IllegalStateException("QLaunchBridge.install was not called")

    /// The app's private files directory, where Rust keeps `qlaunch.json`.
    @JvmStatic
    fun filesDirPath(): String = getAppContext().filesDir.absolutePath

    /// Every launcher activity, as the JSON shape Rust's `parse_list_json` reads.
    /// `selfPackage` lets Rust drop our own package and is the package-name
    /// source of truth when the debug build's applicationId differs.
    @JvmStatic
    fun listLaunchActivitiesJson(): String {
        val context = getAppContext()
        val pm = context.packageManager
        val intent = Intent(Intent.ACTION_MAIN, null).addCategory(Intent.CATEGORY_LAUNCHER)
        val resolved = pm.queryIntentActivities(intent, 0)
        val activities = JSONArray()
        for (ri in resolved) {
            val ai = ri.activityInfo
            val app = ai.applicationInfo ?: continue
            val isGame = (app.flags and ApplicationInfo.FLAG_IS_GAME) != 0 ||
                app.category == ApplicationInfo.CATEGORY_GAME
            activities.put(
                JSONObject()
                    .put("package", ai.packageName)
                    .put("activity", ai.name)
                    .put("label", ai.loadLabel(pm)?.toString() ?: ai.name)
                    .put("isGame", isGame)
            )
        }
        return JSONObject()
            .put("selfPackage", context.packageName)
            .put("activities", activities)
            .toString()
    }

    /// The activity's icon as PNG bytes at `sizePx`, or null if it cannot be had.
    /// Draws through a Canvas rather than casting to BitmapDrawable, so adaptive,
    /// vector and colour icons all work (Review Focus 4).
    @JvmStatic
    fun iconPng(packageName: String, activity: String, sizePx: Int): ByteArray? {
        val context = getAppContext()
        return try {
            val drawable = context.packageManager
                .getActivityIcon(ComponentName(packageName, activity))
            val bitmap = drawableToBitmap(drawable, sizePx)
            ByteArrayOutputStream().use { out ->
                bitmap.compress(Bitmap.CompressFormat.PNG, 100, out)
                out.toByteArray()
            }
        } catch (e: Exception) {
            null
        }
    }

    /// Opens the system's uninstall confirmation for `packageName`.
    @JvmStatic
    fun uninstall(packageName: String) {
        val context = getAppContext()
        val intent = Intent(Intent.ACTION_DELETE, Uri.parse("package:$packageName"))
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        context.startActivity(intent)
    }

    /// Starts the activity. Throws if the component is gone, so Rust can report it.
    @JvmStatic
    fun launch(packageName: String, activity: String) {
        val context = getAppContext()
        val intent = Intent(Intent.ACTION_MAIN).apply {
            addCategory(Intent.CATEGORY_LAUNCHER)
            component = ComponentName(packageName, activity)
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED
        }
        context.startActivity(intent)
    }

    private fun drawableToBitmap(drawable: Drawable, sizePx: Int): Bitmap {
        if (drawable is BitmapDrawable && drawable.bitmap != null) {
            return drawable.bitmap
        }
        val width = if (drawable.intrinsicWidth > 0) drawable.intrinsicWidth else sizePx.coerceAtLeast(1)
        val height = if (drawable.intrinsicHeight > 0) drawable.intrinsicHeight else sizePx.coerceAtLeast(1)
        val bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(bitmap)
        drawable.setBounds(0, 0, canvas.width, canvas.height)
        drawable.draw(canvas)
        return bitmap
    }
}
