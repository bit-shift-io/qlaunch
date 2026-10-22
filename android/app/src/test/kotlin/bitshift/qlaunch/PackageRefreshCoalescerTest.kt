package bitshift.qlaunch

import org.junit.Assert.*
import org.junit.Test

class PackageRefreshCoalescerTest {
    private class Clock {
        var now = 0L
        val tasks = mutableMapOf<Int, Pair<Long, () -> Unit>>()
        private var next = 0
        fun schedule(delay: Long, task: () -> Unit): () -> Unit {
            val id = next++
            tasks[id] = Pair(now + delay, task)
            return { tasks.remove(id); Unit }
        }
        fun advance(ms: Long) {
            val target = now + ms
            while (true) {
                val entry = tasks.minByOrNull { it.value.first } ?: break
                if (entry.value.first > target) break
                tasks.remove(entry.key)
                now = entry.value.first
                entry.value.second()
            }
            now = target
        }
    }

    @Test fun burstsCompleteEveryBroadcastWithOneScan() {
        val clock = Clock()
        var scans = 0
        var finished = 0
        val worker = PackageRefreshCoalescer({ clock.now }, clock::schedule, { scans++ })
        worker.enqueue { finished++ }
        clock.advance(100)
        worker.enqueue { finished++ }
        clock.advance(249)
        assertEquals(0, scans)
        clock.advance(1)
        assertEquals(1, scans)
        assertEquals(2, finished)
        assertTrue(clock.tasks.isEmpty())
    }

    @Test fun continuousEventsFlushByOneSecond() {
        val clock = Clock()
        var finished = 0
        var scans = 0
        val worker = PackageRefreshCoalescer({ clock.now }, clock::schedule, { scans++ })
        repeat(10) { worker.enqueue { finished++ }; clock.advance(100) }
        assertEquals(1, scans)
        assertEquals(10, finished)
    }

    @Test fun failuresFinishAndEventsDuringScanGetAnotherBatch() {
        val clock = Clock()
        var scans = 0
        var finished = 0
        var errors = 0
        lateinit var worker: PackageRefreshCoalescer
        worker = PackageRefreshCoalescer({ clock.now }, clock::schedule, {
            scans++
            if (scans == 1) {
                worker.enqueue { finished++ }
                throw IllegalStateException("scan failed")
            }
        }, { errors++ })
        worker.enqueue { finished++ }
        clock.advance(250)
        assertEquals(1, finished)
        clock.advance(250)
        assertEquals(2, finished)
        assertEquals(2, scans)
        assertEquals(1, errors)
        assertTrue(clock.tasks.isEmpty())
    }

    @Test fun replacingPackageRemovalIsNotARefreshTrigger() {
        val removed = "android.intent.action.PACKAGE_REMOVED"
        assertTrue(isReplacingRemoval(removed, true))
        assertFalse(isReplacingRemoval(removed, false))
        assertFalse(
            isReplacingRemoval("android.intent.action.PACKAGE_ADDED", true),
        )
    }
}
