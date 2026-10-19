package bitshift.qlaunch

import android.content.Intent
import android.os.Bundle
import io.flutter.embedding.android.FlutterActivity

class MainActivity : FlutterActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        // Loaded before the Flutter engine starts so JNI_OnLoad runs on this
        // thread (the app class loader) and Rust can capture the JVM and the
        // bridge class. See §7.3.
        System.loadLibrary("qlaunch_core")
        super.onCreate(savedInstanceState)
        QLaunchBridge.install(applicationContext)
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
