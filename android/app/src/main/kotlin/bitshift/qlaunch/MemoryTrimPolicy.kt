package bitshift.qlaunch

import android.content.ComponentCallbacks2

// UI_HIDDEN is a visibility hint, not low memory. Flutter treats every level
// >= RUNNING_LOW as pressure, so forwarding it discards our warm tile cache.
@Suppress("DEPRECATION")
internal fun shouldForwardMemoryTrim(level: Int): Boolean =
    level != ComponentCallbacks2.TRIM_MEMORY_UI_HIDDEN
