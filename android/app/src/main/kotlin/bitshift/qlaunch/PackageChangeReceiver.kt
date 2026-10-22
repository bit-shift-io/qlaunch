package bitshift.qlaunch

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log
import android.os.SystemClock
import java.util.concurrent.ScheduledThreadPoolExecutor
import java.util.concurrent.TimeUnit

/// Android reports an in-place package update as a removal with
/// `EXTRA_REPLACING` set, then an addition. Treating that removal as an
/// uninstall would delete the app's items and shortcuts before the addition
/// arrives.
internal fun isReplacingRemoval(action: String?, replacing: Boolean): Boolean =
    action == "android.intent.action.PACKAGE_REMOVED" && replacing

/// Tells the core an app was installed, removed or changed, so the launcher
/// updates without a restart.
///
/// Since Android 8 the package broadcasts cannot come from the manifest, so
/// MainActivity registers this at runtime; the manifest declares the receiver
/// for the exempt PACKAGE_FULLY_REMOVED only, which can start the process when
/// it is dead.
class PackageChangeReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (isReplacingRemoval(
                intent.action,
                intent.getBooleanExtra(Intent.EXTRA_REPLACING, false),
            )
        ) {
            return
        }
        val appContext = context.applicationContext
        val pending = goAsync()
        QLaunchBridge.install(appContext)
        coalescer.enqueue { pending.finish() }
    }

    private external fun onPackagesChanged()

    private companion object {
        const val TAG = "QLaunch"
        // One worker for all receiver instances; no periodic jobs, and the
        // worker thread expires when idle rather than keeping a polling loop.
        val executor = ScheduledThreadPoolExecutor(1) { task ->
            Thread(task, "qlaunch-package-refresh").apply { isDaemon = true }
        }.apply {
            removeOnCancelPolicy = true
            setKeepAliveTime(30, TimeUnit.SECONDS)
            allowCoreThreadTimeOut(true)
        }
        val coalescer = PackageRefreshCoalescer(
            SystemClock::uptimeMillis,
            { delay, task ->
                val future = executor.schedule({ task() }, delay, TimeUnit.MILLISECONDS)
                val cancel: () -> Unit = { future.cancel(false); Unit }
                cancel
            },
            {
                System.loadLibrary("qlaunch_core")
                PackageChangeReceiver().onPackagesChanged()
            },
            { error -> Log.w(TAG, "package change refresh failed", error) },
        )
    }
}
