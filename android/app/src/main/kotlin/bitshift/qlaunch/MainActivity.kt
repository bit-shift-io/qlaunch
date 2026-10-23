package bitshift.qlaunch

import android.content.Context
import android.content.ComponentName
import android.app.Activity
import android.content.Intent
import android.content.IntentFilter
import android.net.Uri
import android.os.Bundle
import android.os.Environment
import android.provider.Settings
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.android.FlutterActivityLaunchConfigs
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    private var shortcutGroup: String? = null
    private var wasStopped = false

    override fun configureFlutterEngine(engine: FlutterEngine) {
        super.configureFlutterEngine(engine)
        // "All files access" is a special app-op, not a runtime permission, so
        // it is checked and requested through Settings rather than a dialog.
        MethodChannel(engine.dartExecutor.binaryMessenger, STORAGE_CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "hasAllFilesAccess" -> result.success(Environment.isExternalStorageManager())
                    "requestAllFilesAccess" -> {
                        try {
                            startActivity(
                                Intent(
                                    Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION,
                                    Uri.parse("package:$packageName"),
                                ),
                            )
                            result.success(null)
                        } catch (error: Exception) {
                            startActivity(Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION))
                            result.success(null)
                        }
                    }
                    else -> result.notImplemented()
                }
            }
    }

    private companion object {
        const val STORAGE_CHANNEL = "bitshift.qlaunch/storage"
    }

    @Suppress("DEPRECATION")
    fun beginShortcut(provider: String, group: String) {
        check(shortcutGroup == null) { "A shortcut request is already open" }
        val component = ComponentName.unflattenFromString(provider) ?: error("Invalid shortcut provider")
        shortcutGroup = group
        try {
            startActivityForResult(Intent(Intent.ACTION_CREATE_SHORTCUT).setComponent(component), 1001)
        } catch (error: Exception) {
            shortcutGroup = null
            throw error
        }
    }

    @Deprecated("Activity result bridge")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != 1001) return
        val group = shortcutGroup ?: return
        shortcutGroup = null
        if (resultCode != Activity.RESULT_OK) { onShortcutResult(group, "", ""); return }
        try {
            onShortcutResult(group, LauncherActions.resultJson(data ?: error("Empty shortcut result")), "")
        } catch (error: Exception) {
            onShortcutResult(group, "", error.message ?: "Could not create shortcut")
        }
    }

    private external fun onShortcutResult(group: String, json: String, error: String)
    /// A launcher shows the system wallpaper behind its own UI (§7.7). Android
    /// 14+ refuses to hand an app the wallpaper bytes without a media
    /// permission, so the wallpaper is shown the way every launcher does it:
    /// the window is transparent where Flutter is, and `windowShowWallpaper`
    /// puts the wallpaper underneath.
    override fun getBackgroundMode(): FlutterActivityLaunchConfigs.BackgroundMode =
        FlutterActivityLaunchConfigs.BackgroundMode.transparent

    private val packageReceiver = PackageChangeReceiver()

    override fun onCreate(savedInstanceState: Bundle?) {
        // Loaded before the Flutter engine starts so JNI_OnLoad runs on this
        // thread (the app class loader) and Rust can capture the JVM and the
        // bridge class. See §7.3.
        System.loadLibrary("qlaunch_core")
        super.onCreate(savedInstanceState)
        // Extend the wallpaper/background behind transparent system bars.
        // Flutter's SafeArea still keeps controls clear of their insets.
        window.setDecorFitsSystemWindows(false)
        QLaunchBridge.install(applicationContext)
        LauncherActions.attach(this)

        // Android 8+ (API 26) will not deliver these implicit broadcasts to a
        // manifest-declared receiver, so they are registered here at runtime
        // for as long as the process lives. The manifest keeps only the exempt
        // PACKAGE_FULLY_REMOVED as a backstop for when the process is dead.
        registerReceiver(
            packageReceiver,
            IntentFilter().apply {
                addAction(Intent.ACTION_PACKAGE_ADDED)
                addAction(Intent.ACTION_PACKAGE_REMOVED)
                addAction(Intent.ACTION_PACKAGE_REPLACED)
                addAction(Intent.ACTION_PACKAGE_CHANGED)
                addDataScheme("package")
            },
            Context.RECEIVER_NOT_EXPORTED,
        )
    }

    override fun onDestroy() {
        LauncherActions.detach(this)
        unregisterReceiver(packageReceiver)
        super.onDestroy()
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) LauncherActions.reapplyDisplaySettings(this)
    }

    override fun onStop() {
        wasStopped = true
        super.onStop()
    }

    override fun onTrimMemory(level: Int) {
        // Flutter's onStop/onResume still handle renderer lifecycle cleanup.
        // Keep the bounded working set on app switches; forward actual pressure
        // (and all other levels) through the usual engine/plugin path.
        if (shouldForwardMemoryTrim(level)) super.onTrimMemory(level)
    }

    override fun onResume() {
        super.onResume()
        wasStopped = false
    }

    /// A Home intent that brings us back from another app only reveals the
    /// remembered tab. Android also pauses a foreground activity before
    /// onNewIntent, so onPause cannot distinguish that from a repeated Home;
    /// onStop can. Returning via Back resets this state in onResume as well.
    ///
    /// While the notification shade (or another system overlay) is open this
    /// window does not have focus, and the Home press is what dismisses the
    /// overlay. Dart is told so separately: that press returns to the
    /// launcher but never advances to the search.
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        if (intent.action == Intent.ACTION_MAIN &&
            intent.hasCategory(Intent.CATEGORY_HOME) && !wasStopped) {
            if (hasWindowFocus()) onHomePressed() else onHomePressedFromOverlay()
        }
    }

    /// Dart closes navigation history/search or cycles left through tabs. The
    /// manifest opts out of predictive back, so this is always the path.
    @Suppress("DEPRECATION")
    override fun onBackPressed() {
        onBackPressedEvent()
    }

    private external fun onHomePressed()

    private external fun onHomePressedFromOverlay()

    private external fun onBackPressedEvent()
}
