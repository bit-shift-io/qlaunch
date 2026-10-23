package bitshift.qlaunch

import android.app.Activity
import android.accessibilityservice.AccessibilityService
import android.app.admin.DevicePolicyManager
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.ActivityInfo
import android.view.WindowInsets
import android.view.WindowInsetsController
import android.graphics.Bitmap
import android.net.Uri
import android.os.Looper
import android.provider.Settings
import android.util.Base64
import android.widget.Toast
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.lang.ref.WeakReference
import java.util.concurrent.FutureTask

/// OS operations that require the current Activity or a result round-trip.
object LauncherActions {
    private var displaySettings: String? = null

    fun applyDisplaySettings(json: String) {
        displaySettings = json
        onMain { applyWindow(it, json) }
    }

    fun reapplyDisplaySettings(host: MainActivity) {
        displaySettings?.let { applyWindow(host, it) }
    }

    private fun applyWindow(host: MainActivity, json: String) {
        val data = JSONObject(json)
        val orientation = when (data.getString("orientation")) {
            "portrait" -> ActivityInfo.SCREEN_ORIENTATION_PORTRAIT
            "landscape" -> ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE
            else -> ActivityInfo.SCREEN_ORIENTATION_FULL_SENSOR
        }
        if (host.requestedOrientation != orientation) host.requestedOrientation = orientation
        host.window.insetsController?.let { controller ->
            controller.systemBarsBehavior = WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
            if (data.getBoolean("status_bar_visible")) controller.show(WindowInsets.Type.statusBars())
            else controller.hide(WindowInsets.Type.statusBars())
        }
    }
    private var activity = WeakReference<MainActivity>(null)
    fun attach(value: MainActivity) { activity = WeakReference(value) }
    fun detach(value: MainActivity) { if (activity.get() === value) activity.clear() }

    fun onMain(action: (MainActivity) -> Unit) {
        val host = activity.get() ?: error("Launcher activity is not available")
        if (Looper.myLooper() == Looper.getMainLooper()) action(host)
        else {
            val task = FutureTask { action(host) }
            host.runOnUiThread(task)
            task.get()
        }
    }

    fun lockScreen() = onMain { host ->
        // Revoke the legacy permission on upgrade; never fall back to lockNow(),
        // which forces PIN/password authentication on the next unlock.
        val manager = host.getSystemService(Context.DEVICE_POLICY_SERVICE) as DevicePolicyManager
        val admin = ComponentName(host, LockDeviceAdminReceiver::class.java)
        if (manager.isAdminActive(admin)) manager.removeActiveAdmin(admin)

        val service = ScreenLockService.instance
        if (service != null) {
            check(service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_LOCK_SCREEN)) {
                "Android could not lock the screen. Please try again."
            }
        } else {
            Toast.makeText(host, R.string.screen_lock_service_enable, Toast.LENGTH_LONG).show()
            host.startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))
        }
    }

    fun openNotifications() = onMain { host ->
        val service = ScreenLockService.instance
        if (service != null) {
            check(service.performGlobalAction(AccessibilityService.GLOBAL_ACTION_NOTIFICATIONS)) {
                "Android could not open notifications. Please try again."
            }
        } else {
            Toast.makeText(host, R.string.screen_lock_service_enable, Toast.LENGTH_LONG).show()
            host.startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))
        }
    }

    fun providersJson(): String {
        val context = QLaunchBridge.getAppContext()
        val pm = context.packageManager
        val result = JSONArray()
        pm.queryIntentActivities(Intent(Intent.ACTION_CREATE_SHORTCUT), 0)
            .sortedBy { it.loadLabel(pm).toString().lowercase() }
            .forEach { ri ->
                result.put(JSONObject().put("id", ComponentName(ri.activityInfo.packageName, ri.activityInfo.name).flattenToString())
                    .put("label", ri.loadLabel(pm).toString()))
            }
        return result.toString()
    }

    fun launchShortcut(json: String) {
        val data = JSONObject(json)
        fun value(key: String): String? = if (data.isNull(key)) null else data.getString(key)
        val intent = Intent().apply {
            action = value("action")
            val uri = value("data")?.let(Uri::parse)
            val mime = value("mime_type")
            setDataAndType(uri, mime)
            value("package")?.let { setPackage(it) }
            value("component")?.let {
                component = ComponentName.unflattenFromString(it) ?: error("Invalid shortcut component")
            }
            flags = data.optInt("flags") or Intent.FLAG_ACTIVITY_NEW_TASK
            data.optJSONArray("categories")?.let { categories ->
                for (i in 0 until categories.length()) addCategory(categories.getString(i))
            }
            data.optJSONArray("extras")?.let { extras ->
                for (i in 0 until extras.length()) {
                    val extra = extras.getJSONObject(i)
                    val key = extra.getString("key")
                    val raw = extra.getString("value")
                    when (extra.getString("kind")) {
                        "string" -> putExtra(key, raw)
                        "bool" -> putExtra(key, raw.toBooleanStrict())
                        "int" -> putExtra(key, raw.toInt())
                        else -> error("Unsupported shortcut extra")
                    }
                }
            }
        }
        QLaunchBridge.getAppContext().startActivity(intent)
    }

    @Suppress("DEPRECATION")
    fun resultJson(data: Intent): String {
        val intent = data.getParcelableExtra(Intent.EXTRA_SHORTCUT_INTENT) as? Intent
            ?: error("Shortcut provider returned no intent")
        val label = data.getStringExtra(Intent.EXTRA_SHORTCUT_NAME)?.trim()
            ?.takeIf { it.isNotEmpty() } ?: error("Shortcut provider returned no name")
        val extras = JSONArray()
        intent.extras?.let { bundle ->
            for (key in bundle.keySet()) {
                val raw = bundle.get(key)
                val kind = when (raw) {
                    is String -> "string"
                    is Boolean -> "bool"
                    is Int -> "int"
                    else -> error("Unsupported shortcut extra: $key")
                }
                extras.put(JSONObject().put("key", key).put("kind", kind).put("value", raw.toString()))
            }
        }
        val payload = JSONObject().put("type", "android")
            .put("action", intent.action ?: JSONObject.NULL)
            .put("data", intent.dataString ?: JSONObject.NULL)
            .put("mime_type", intent.type ?: JSONObject.NULL)
            .put("package", intent.`package` ?: JSONObject.NULL)
            .put("component", intent.component?.flattenToString() ?: JSONObject.NULL)
            .put("flags", intent.flags)
            .put("categories", JSONArray(intent.categories?.toList() ?: emptyList<String>()))
            .put("extras", extras)

        // A provider may give us the icon as a bitmap, or as a reference to one
        // of its own drawables. The reference stays small and can be resolved
        // again after a restore; only a bitmap has to be stored.
        val bitmap = data.getParcelableExtra(Intent.EXTRA_SHORTCUT_ICON) as? Bitmap
        val resource = if (bitmap == null) {
            data.getParcelableExtra(Intent.EXTRA_SHORTCUT_ICON_RESOURCE)
                as? Intent.ShortcutIconResource
        } else null
        val iconRef = resource?.let { QLaunchBridge.iconRefFor(it) }
        val icon = bitmap?.let {
            val scaled = if (it.width > 192 || it.height > 192) {
                val ratio = minOf(192.0 / it.width, 192.0 / it.height)
                Bitmap.createScaledBitmap(it, (it.width * ratio).toInt().coerceAtLeast(1),
                    (it.height * ratio).toInt().coerceAtLeast(1), true)
            } else it
            val bytes = ByteArrayOutputStream()
            scaled.compress(Bitmap.CompressFormat.PNG, 100, bytes)
            JSONArray().apply { bytes.toByteArray().forEach { byte -> put(byte.toInt() and 255) } }
        }
        return JSONObject().put("label", label).put("payload", payload)
            .put("icon", icon ?: JSONObject.NULL)
            .put("icon_ref", iconRef ?: JSONObject.NULL).toString()
    }
}
