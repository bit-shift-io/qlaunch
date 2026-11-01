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

    /**
     * Lifecycle diagnostics. Each line names the activity instance window, so a
     * destroyed instance's late callbacks (which once left the launcher with no
     * iced window: a frozen or blank Home) are visible in logcat next to the
     * replacement instance's. Filter with `adb logcat -s qlaunch-iced:*`.
     */
    private fun logLifecycle(event: String) {
        android.util.Log.i(
            "qlaunch-iced",
            "lifecycle $event focus=${hasWindowFocus()} stopped=$wasStopped " +
                "finishing=$isFinishing destroyed=$isDestroyed " +
                "window=${System.identityHashCode(window)}",
        )
    }

    fun publishAccessibility(json: String) {
        if (!ACCESSIBILITY_EXPOSURE_ENABLED) return
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
        logLifecycle("onCreate")

        // No host view is attached while exposure is off: with nothing to
        // publish into, the native side's publishes are dropped in
        // `LauncherActions.publishAccessibility`.
        if (ACCESSIBILITY_EXPOSURE_ENABLED) {
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
        }

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

    override fun onStart() {
        logLifecycle("onStart enter")
        super.onStart()
        logLifecycle("onStart exit")
    }

    override fun onPause() {
        logLifecycle("onPause enter")
        super.onPause()
        logLifecycle("onPause exit")
    }

    override fun onDestroy() {
        logLifecycle("onDestroy enter")
        LauncherActions.detach(this)
        unregisterReceiver(packageReceiver)
        super.onDestroy()
        logLifecycle("onDestroy exit")
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        logLifecycle("onWindowFocusChanged $hasFocus")
        if (hasFocus) {
            // The native surface is created after onCreate and is added above
            // this view. TalkBack sends hover events to the topmost child, so
            // lift the accessibility host again once the window is ready.
            if (::accessibility.isInitialized) accessibility.bringToFront()
            LauncherActions.reapplyDisplaySettings(this)
        }
    }

    override fun onStop() {
        logLifecycle("onStop enter")
        wasStopped = true
        super.onStop()
        logLifecycle("onStop exit")
    }

    override fun onResume() {
        logLifecycle("onResume enter")
        super.onResume()
        wasStopped = false
        logLifecycle("onResume exit")
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
        logLifecycle("onNewIntent ${intent.action}")
        if (intent.action == Intent.ACTION_MAIN &&
            intent.hasCategory(Intent.CATEGORY_HOME) && !wasStopped) {
            if (hasWindowFocus()) onHomePressed() else onHomePressedFromOverlay()
        }
    }

    /// Memory pressure can take the surface away without a foreground change;
    /// note it so a later blank screen can be told from a lifecycle race.
    override fun onLowMemory() {
        super.onLowMemory()
        logLifecycle("onLowMemory")
    }

    override fun onTrimMemory(level: Int) {
        super.onTrimMemory(level)
        logLifecycle("onTrimMemory $level")
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
