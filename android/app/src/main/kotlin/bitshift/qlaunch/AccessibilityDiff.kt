package bitshift.qlaunch

import kotlin.math.min

data class NodeRect(val left: Int, val top: Int, val right: Int, val bottom: Int)

data class NodeSnapshot(
    val id: Int,
    val role: String,
    val bounds: NodeRect,
    val label: String,
    val value: String?,
    val selected: Boolean,
    val enabled: Boolean,
    val actions: List<String>,
)

data class TreeSnapshot(
    val screen: String,
    val order: List<Int>,
    val nodes: Map<Int, NodeSnapshot>,
)

data class TreeDiff(
    val invalidateRoot: Boolean,
    val invalidated: Set<Int>,
    /// Ids that left the tree. The platform must not be asked to populate
    /// these; ExploreByTouchHelper crashes on a virtual view it cannot find.
    val removed: Set<Int>,
    val textChanged: Set<Int>,
    val focusTarget: Int?,
)

/// The platform caches the virtual tree; a publish only reports what changed.
/// Keeping the focused id across an update is deliberate: invalidating the root
/// on every keystroke or page step would drop TalkBack focus and replay the
/// whole screen.
fun diffTrees(old: TreeSnapshot?, new: TreeSnapshot, focusedId: Int?): TreeDiff {
    if (old == null || old.screen != new.screen) {
        return TreeDiff(
            invalidateRoot = true,
            invalidated = emptySet(),
            removed = emptySet(),
            textChanged = emptySet(),
            focusTarget = if (focusedId != null && focusedId !in new.nodes) new.order.firstOrNull() else null,
        )
    }

    val invalidated = linkedSetOf<Int>()
    val removed = linkedSetOf<Int>()
    val textChanged = linkedSetOf<Int>()

    for (id in old.order) if (id !in new.nodes) removed.add(id)
    for (id in new.order) if (id !in old.nodes) invalidated.add(id)

    val oldIndex = old.order.withIndex().associate { (index, id) -> id to index }
    val newIndex = new.order.withIndex().associate { (index, id) -> id to index }
    for (id in new.order) {
        val before = old.nodes[id] ?: continue
        val after = new.nodes.getValue(id)
        val movedOrChanged = oldIndex[id] != newIndex[id] ||
            before.bounds != after.bounds ||
            before.selected != after.selected ||
            before.enabled != after.enabled ||
            before.actions != after.actions
        when {
            movedOrChanged -> invalidated.add(id)
            before.label != after.label || before.value != after.value -> textChanged.add(id)
        }
    }

    if (focusedId == null || focusedId in new.nodes) {
        return TreeDiff(false, invalidated, removed, textChanged, focusTarget = null)
    }

    if (new.order.isEmpty()) {
        return TreeDiff(
            invalidateRoot = true,
            invalidated = emptySet(),
            removed = removed,
            textChanged = textChanged,
            focusTarget = null,
        )
    }

    // Prefer the next surviving node, then the one before.
    val lostIndex = oldIndex[focusedId] ?: 0
    val next = old.order.drop(min(lostIndex, old.order.size)).firstOrNull { it in new.nodes }
    val previous = old.order.take(lostIndex).lastOrNull { it in new.nodes }
    return TreeDiff(
        invalidateRoot = false,
        invalidated = invalidated,
        removed = removed,
        textChanged = textChanged,
        focusTarget = next ?: previous,
    )
}
