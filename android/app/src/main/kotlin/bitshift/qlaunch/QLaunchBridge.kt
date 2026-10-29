package bitshift.qlaunch

import android.content.ComponentName
import android.content.ContentUris
import android.content.ContentValues
import android.content.Context
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.net.Uri
import android.provider.MediaStore
import android.util.Base64
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.drawable.Drawable
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import kotlin.math.roundToInt

/// Everything the Rust core asks the OS for. Rust owns all data and logic; this
/// class only answers questions and performs OS actions. Methods are `@JvmStatic`
/// so Rust finds them as static methods on `bitshift/qlaunch/QLaunchBridge`.
object QLaunchBridge {
    @JvmStatic fun lockScreen() = LauncherActions.lockScreen()
    @JvmStatic fun openNotifications() = LauncherActions.openNotifications()
    @JvmStatic fun shortcutProvidersJson(): String = LauncherActions.providersJson()
    @JvmStatic fun beginShortcut(provider: String, group: String) =
        LauncherActions.onMain { it.beginShortcut(provider, group) }
    @JvmStatic fun launchShortcut(json: String) = LauncherActions.launchShortcut(json)
    @JvmStatic fun applyDisplaySettings(json: String) = LauncherActions.applyDisplaySettings(json)
    @JvmStatic fun publishAccessibility(json: String) = LauncherActions.publishAccessibility(json)
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

    private fun downloadsCollection(): Uri =
        MediaStore.Downloads.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)

    /// Writes a config backup into the shared Downloads collection, visible in
    /// Files and any other app. MediaStore needs no storage permission for the
    /// app's own files. The name is validated by Rust before it gets here.
    @JvmStatic
    fun writeBackup(fileName: String, body: String) {
        val resolver = getAppContext().contentResolver
        val collection = downloadsCollection()
        resolver.delete(
            collection,
            "${MediaStore.Downloads.DISPLAY_NAME} = ?",
            arrayOf(fileName),
        )
        val values = ContentValues().apply {
            put(MediaStore.Downloads.DISPLAY_NAME, fileName)
            put(MediaStore.Downloads.MIME_TYPE, "application/json")
            put(MediaStore.Downloads.IS_PENDING, 1)
        }
        val uri = resolver.insert(collection, values) ?: error("Could not create $fileName")
        try {
            resolver.openOutputStream(uri)?.use { it.write(body.toByteArray(Charsets.UTF_8)) }
                ?: error("Could not write $fileName")
            values.clear()
            values.put(MediaStore.Downloads.IS_PENDING, 0)
            resolver.update(uri, values, null, null)
        } catch (error: Exception) {
            resolver.delete(uri, null, null)
            throw error
        }
    }

    /// The names of this app's config backups in Downloads, as a JSON array.
    /// Rust filters the names to the exact backup shape and sorts them.
    @JvmStatic
    fun listBackups(): String {
        val resolver = getAppContext().contentResolver
        val names = JSONArray()
        resolver.query(
            downloadsCollection(),
            arrayOf(MediaStore.Downloads.DISPLAY_NAME),
            "${MediaStore.Downloads.DISPLAY_NAME} LIKE ?",
            arrayOf("qlaunch-backup-%"),
            null,
        )?.use { cursor ->
            val index = cursor.getColumnIndexOrThrow(MediaStore.Downloads.DISPLAY_NAME)
            while (cursor.moveToNext()) names.put(cursor.getString(index))
        }
        return names.toString()
    }

    /// Reads one config backup back as text. Throws if it is not there.
    @JvmStatic
    fun readBackup(fileName: String): String {
        val resolver = getAppContext().contentResolver
        val collection = downloadsCollection()
        resolver.query(
            collection,
            arrayOf(MediaStore.Downloads._ID),
            "${MediaStore.Downloads.DISPLAY_NAME} = ?",
            arrayOf(fileName),
            null,
        )?.use { cursor ->
            if (cursor.moveToFirst()) {
                val id = cursor.getLong(cursor.getColumnIndexOrThrow(MediaStore.Downloads._ID))
                val uri = ContentUris.withAppendedId(collection, id)
                resolver.openInputStream(uri)?.use { stream ->
                    return stream.readBytes().toString(Charsets.UTF_8)
                }
            }
        }
        error("No backup named $fileName")
    }

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
        // One PackageInfo lookup per package, not per activity. The update time
        // is what lets the icon cache tell a new version's icon from the old.
        val updateTimes = HashMap<String, Long>()
        for (ri in resolved) {
            val ai = ri.activityInfo
            val app = ai.applicationInfo ?: continue
            val isGame = (app.flags and ApplicationInfo.FLAG_IS_GAME) != 0 ||
                app.category == ApplicationInfo.CATEGORY_GAME
            val lastUpdateTime = updateTimes.getOrPut(ai.packageName) {
                try {
                    pm.getPackageInfo(ai.packageName, 0).lastUpdateTime
                } catch (e: Exception) {
                    0L
                }
            }
            activities.put(
                JSONObject()
                    .put("package", ai.packageName)
                    .put("activity", ai.name)
                    .put("label", ai.loadLabel(pm)?.toString() ?: ai.name)
                    .put("isGame", isGame)
                    .put("lastUpdateTime", lastUpdateTime)
            )
        }
        return JSONObject()
            .put("selfPackage", context.packageName)
            .put("activities", activities)
            .toString()
    }

    private const val ICON_REF_PREFIX = "android-res://"

    /// A stable reference to a drawable another app supplied as a shortcut icon.
    @JvmStatic
    fun iconRefFor(resource: Intent.ShortcutIconResource): String? {
        if (resource.packageName.isBlank() || resource.resourceName.isBlank()) return null
        return ICON_REF_PREFIX + resource.packageName + "/" + resource.resourceName
    }

    /// Draws the drawable named by `reference` and returns it as base64 PNG.
    /// Returns null if the app or resource is gone.
    @JvmStatic
    fun resourceIconPng(reference: String, sizePx: Int): String? {
        if (!reference.startsWith(ICON_REF_PREFIX)) return null
        val rest = reference.removePrefix(ICON_REF_PREFIX)
        val slash = rest.indexOf('/')
        if (slash <= 0 || slash == rest.length - 1) return null
        val packageName = rest.substring(0, slash)
        val resourceName = rest.substring(slash + 1)
        return try {
            val resources = getAppContext().packageManager
                .getResourcesForApplication(packageName)
            val id = if (resourceName.contains(':')) {
                resources.getIdentifier(resourceName, null, null)
            } else {
                resources.getIdentifier(resourceName, null, packageName)
            }
            if (id == 0) return null
            val bitmap = drawableToBitmap(resources.getDrawable(id, null), sizePx)
            val bytes = ByteArrayOutputStream()
            bitmap.compress(Bitmap.CompressFormat.PNG, 100, bytes)
            Base64.encodeToString(bytes.toByteArray(), Base64.NO_WRAP)
        } catch (e: Exception) {
            null
        }
    }

    /// The activity's icon as raw pixels, or null if it cannot be had.
    ///
    /// The result is `[width, height, argb, argb, ...]`: two leading size slots
    /// then `Bitmap.getPixels` output (`0xAARRGGBB` per pixel). Rust turns the
    /// pixels into RGBA bytes, so no PNG is ever encoded or decoded. Drawing
    /// through a Canvas rather than casting to BitmapDrawable is what lets
    /// adaptive, vector and colour icons all work (Review Focus 4).
    @JvmStatic
    fun iconRgba(packageName: String, activity: String, sizePx: Int): IntArray? {
        val context = getAppContext()
        return try {
            val drawable = context.packageManager
                .getActivityIcon(ComponentName(packageName, activity))
            val bitmap = drawableToBitmap(drawable, sizePx)
            val width = bitmap.width
            val height = bitmap.height
            val out = IntArray(2 + width * height)
            out[0] = width
            out[1] = height
            bitmap.getPixels(out, 2, width, 0, 0, width, height)
            out
        } catch (e: Exception) {
            null
        }
    }

    /// Opens the system wallpaper chooser. There is no result the app needs:
    /// the window already shows the system wallpaper (theme
    /// `windowShowWallpaper`), so the change is visible as soon as the system
    /// applies it.
    @JvmStatic
    fun chooseWallpaper() {
        val context = getAppContext()
        val intent = Intent(Intent.ACTION_SET_WALLPAPER)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        context.startActivity(
            Intent.createChooser(intent, "Choose wallpaper")
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        )
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

    /// Draws `drawable` into a fresh bitmap no larger than `sizePx` on either
    /// side, at the icon's aspect ratio.
    ///
    /// Scaling here (rather than returning the intrinsic bitmap) is what keeps
    /// the pixels crossing the bridge proportional to the size the grid actually
    /// draws. The widget centres the result with `BoxFit.contain`.
    internal fun drawableToBitmap(drawable: Drawable, sizePx: Int): Bitmap {
        // Cap the request so a bogus size cannot overflow the pixel count or
        // allocate an enormous bitmap.
        val target = sizePx.coerceIn(1, 1024)
        val intrinsicWidth =
            if (drawable.intrinsicWidth > 0) drawable.intrinsicWidth else target
        val intrinsicHeight =
            if (drawable.intrinsicHeight > 0) drawable.intrinsicHeight else target
        val scale = minOf(
            target.toFloat() / intrinsicWidth,
            target.toFloat() / intrinsicHeight,
        )
        val width = (intrinsicWidth * scale).roundToInt().coerceAtLeast(1)
        val height = (intrinsicHeight * scale).roundToInt().coerceAtLeast(1)
        val bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(bitmap)
        drawable.setBounds(0, 0, width, height)
        drawable.draw(canvas)
        return bitmap
    }
}
