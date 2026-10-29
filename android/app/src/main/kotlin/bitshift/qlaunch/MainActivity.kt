package bitshift.qlaunch

import android.content.Context
import android.content.ComponentName
import android.app.Activity
import android.content.Intent
import android.content.IntentFilter
import android.os.Bundle
import android.graphics.PixelFormat
import android.view.ViewGroup
import io.github.santuzius.icedandroid.IcedActivity

class MainActivity : IcedActivity() {
    private var shortcutGroup: String? = null
    private var wasStopped = false
    private lateinit var accessibility: LauncherAccessibilityView

    fun publishAccessibility(json: String) {
        if (!::accessibility.isInitialized) return
        if (android.os.Looper.myLooper() != android.os.Looper.getMainLooper()) {
            runOnUiThread { publishAccessibility(json) }
            return
        }
        accessibility.publish(json)
    }

    private fun sendAccessibilityAction(nodeId: Int, action: String, value: String, revision: Long) {
        onAccessibilityAction(nodeId, action, value, revision)
    }

    private external fun onAccessibilityAction(
        nodeId: Int,
        action: String,
        value: String,
        revision: Long,
    )

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
    private val packageReceiver = PackageChangeReceiver()

    override fun onCreate(savedInstanceState: Bundle?) {
        // Install the OS adapter before NativeActivity starts android_main.
        // Loading on the Java thread lets core's JNI_OnLoad cache our class.
        System.loadLibrary("qlaunch_iced")
        QLaunchBridge.install(applicationContext)
        LauncherActions.attach(this)
        super.onCreate(savedInstanceState)
        // NativeActivity.onCreate resets this to RGB_565. Set it afterwards,
        // before the surface is created, or Vulkan's inherited alpha is opaque.
        window.setFormat(PixelFormat.RGBA_8888)

        accessibility = LauncherAccessibilityView(this)
        accessibility.onAction = ::sendAccessibilityAction
        addContentView(
            accessibility,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
        LauncherActions.flushAccessibility(this)

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
        if (hasFocus) {
            // The native surface is created after onCreate and is added above
            // this view. TalkBack sends hover events to the topmost child, so
            // lift the accessibility host again once the window is ready.
            if (::accessibility.isInitialized) accessibility.bringToFront()
            LauncherActions.reapplyDisplaySettings(this)
        }
    }

    override fun onStop() {
        wasStopped = true
        super.onStop()
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

    /// Older back dispatch; IcedActivity handles predictive back through iced.
    @Suppress("DEPRECATION")
    override fun onBackPressed() {
        onBackPressedEvent()
    }

    private external fun onHomePressed()

    private external fun onHomePressedFromOverlay()

    private external fun onBackPressedEvent()
}
