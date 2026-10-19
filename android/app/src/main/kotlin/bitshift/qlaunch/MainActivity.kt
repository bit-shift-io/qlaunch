package bitshift.qlaunch

import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Bundle
import io.flutter.embedding.android.FlutterActivity

class MainActivity : FlutterActivity() {
    private val packageReceiver = PackageChangeReceiver()

    override fun onCreate(savedInstanceState: Bundle?) {
        // Loaded before the Flutter engine starts so JNI_OnLoad runs on this
        // thread (the app class loader) and Rust can capture the JVM and the
        // bridge class. See §7.3.
        System.loadLibrary("qlaunch_core")
        super.onCreate(savedInstanceState)
        QLaunchBridge.install(applicationContext)

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
        unregisterReceiver(packageReceiver)
        super.onDestroy()
    }

    /// When QLaunch is the home app, pressing Home delivers a new MAIN/HOME
    /// intent to this running, `singleTask` activity.
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        onHomePressed()
    }

    /// Back does nothing on the launcher: the old app swallowed it too. The
    /// manifest opts out of predictive back, so this is always the path.
    @Suppress("DEPRECATION")
    override fun onBackPressed() {
        onBackPressedEvent()
    }

    private external fun onHomePressed()

    private external fun onBackPressedEvent()
}
