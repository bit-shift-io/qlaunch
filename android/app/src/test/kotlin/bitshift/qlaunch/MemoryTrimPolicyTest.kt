package bitshift.qlaunch

import android.content.ComponentCallbacks2
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

@Suppress("DEPRECATION")
class MemoryTrimPolicyTest {
    @Test fun hidingTheLauncherDoesNotSignalLowMemory() {
        assertFalse(shouldForwardMemoryTrim(ComponentCallbacks2.TRIM_MEMORY_UI_HIDDEN))
    }

    @Test fun realPressureAndOtherTrimLevelsStillReachFlutter() {
        val levels = listOf(
            ComponentCallbacks2.TRIM_MEMORY_RUNNING_MODERATE,
            ComponentCallbacks2.TRIM_MEMORY_RUNNING_LOW,
            ComponentCallbacks2.TRIM_MEMORY_RUNNING_CRITICAL,
            ComponentCallbacks2.TRIM_MEMORY_BACKGROUND,
            ComponentCallbacks2.TRIM_MEMORY_MODERATE,
            ComponentCallbacks2.TRIM_MEMORY_COMPLETE,
            0,
            100,
        )
        for (level in levels) {
            assertTrue("Trim level $level must still reach Flutter", shouldForwardMemoryTrim(level))
        }
    }
}
