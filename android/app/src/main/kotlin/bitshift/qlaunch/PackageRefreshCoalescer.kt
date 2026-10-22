package bitshift.qlaunch

/// A trailing debounce with bounded delay and exactly-once broadcast completion.
/// Callbacks are injected so the timing policy is tested without Android threads.
internal class PackageRefreshCoalescer(
    private val nowMs: () -> Long,
    private val schedule: (Long, () -> Unit) -> (() -> Unit),
    private val refresh: () -> Unit,
    private val onError: (Throwable) -> Unit = {},
) {
    private val lock = Any()
    private val pending = mutableListOf<() -> Unit>()
    private var firstAt = 0L
    private var lastAt = 0L
    private var cancelScheduled: (() -> Unit)? = null
    private var running = false
    private var generation = 0L

    fun enqueue(finish: () -> Unit) = synchronized(lock) {
        val now = nowMs()
        if (pending.isEmpty()) firstAt = now
        lastAt = now
        pending.add(finish)
        if (!running) scheduleLocked()
    }

    private fun scheduleLocked() {
        cancelScheduled?.invoke()
        val token = ++generation
        val delay = (minOf(lastAt + 250, firstAt + 1000) - nowMs()).coerceAtLeast(0)
        cancelScheduled = schedule(delay) { flush(token) }
    }

    private fun flush(token: Long) {
        val batch = synchronized(lock) {
            if (token != generation || running || pending.isEmpty()) return
            cancelScheduled = null
            running = true
            pending.toList().also { pending.clear() }
        }
        try {
            refresh()
        } catch (error: Throwable) {
            onError(error)
        } finally {
            // A failed scan still owes every OS broadcast its completion.
            for (finish in batch) {
                try { finish() } catch (error: Throwable) { onError(error) }
            }
            synchronized(lock) {
                running = false
                if (pending.isNotEmpty()) scheduleLocked()
            }
        }
    }
}
