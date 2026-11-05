package bitshift.qlaunch

import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test
import java.util.Calendar
import java.util.TimeZone

class DashboardAllDayTest {
    private lateinit var previous: TimeZone

    @Before fun saveDefaultZone() {
        previous = TimeZone.getDefault()
    }

    @After fun restoreDefaultZone() {
        TimeZone.setDefault(previous)
    }

    private fun utcMidnight(year: Int, month: Int, day: Int): Long =
        Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
            clear()
            set(year, month, day, 0, 0, 0)
        }.timeInMillis

    private fun localMidnight(year: Int, month: Int, day: Int): Long =
        Calendar.getInstance().apply {
            clear()
            set(year, month, day, 0, 0, 0)
        }.timeInMillis

    private fun assertNormalised(zone: String, year: Int, month: Int, day: Int) {
        // The ZoneId's rules apply because the helper rebuilds the date through
        // the default calendar, so this also covers DST boundaries.
        TimeZone.setDefault(TimeZone.getTimeZone(zone))
        assertEquals(
            localMidnight(year, month, day),
            DashboardAdapter.allDayLocalMidnight(utcMidnight(year, month, day)),
        )
    }

    @Test fun aheadOfUtcEndsAtLocalMidnightRatherThanNextMorning() {
        // Adelaide (UTC+10:30 in November): the raw UTC end lands ~10:30 the
        // following morning, so it must become the local midnight that closes
        // the event's own day.
        assertNormalised("Australia/Adelaide", 2026, Calendar.NOVEMBER, 6)
    }

    @Test fun behindUtcAlsoAnchorsToLocalMidnight() {
        assertNormalised("America/New_York", 2026, Calendar.NOVEMBER, 6)
    }
}
