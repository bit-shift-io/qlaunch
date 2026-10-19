package bitshift.qlaunch

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

/// Tells the core an app was installed, removed or changed, so the launcher
/// updates without a restart.
///
/// Since Android 8 the package broadcasts cannot come from the manifest, so
/// MainActivity registers this at runtime; the manifest declares the receiver
/// for the exempt PACKAGE_FULLY_REMOVED only, which can start the process when
/// it is dead.
class PackageChangeReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        // The scan and the store write are blocking; keep them off the main
        // thread so a large package set cannot jank or ANR the launcher.
        val appContext = context.applicationContext
        val pending = goAsync()
        Thread {
            try {
                // A package change can start this process on its own, before
                // MainActivity has ever run, so load the library and install
                // the bridge here too. Both are no-ops when already up.
                System.loadLibrary("qlaunch_core")
                QLaunchBridge.install(appContext)
                onPackagesChanged()
            } catch (t: Throwable) {
                Log.w(TAG, "package change refresh failed", t)
            } finally {
                pending.finish()
            }
        }.start()
    }

    private external fun onPackagesChanged()

    private companion object {
        const val TAG = "QLaunch"
    }
}
