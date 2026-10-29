package bitshift.qlaunch

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class AccessibilityDiffTest {
    private fun node(
        id: Int,
        index: Int = 0,
        left: Int = 0,
        top: Int = index * 100,
        right: Int = 100,
        bottom: Int = top + 100,
        label: String = "Node $id",
        value: String? = null,
        selected: Boolean = false,
        enabled: Boolean = true,
        actions: List<String> = listOf("click"),
    ) = NodeSnapshot(
        id,
        "button",
        NodeRect(left, top, right, bottom),
        label,
        value,
        selected,
        enabled,
        actions,
    )

    private fun tree(screen: String, vararg nodes: NodeSnapshot) = TreeSnapshot(
        screen,
        nodes.map { it.id },
        nodes.associateBy { it.id },
    )

    @Test fun screenChangeInvalidatesRoot() {
        val old = tree("launcher", node(1))
        val new = tree("search", node(1))
        val diff = diffTrees(old, new, focusedId = 1)
        assertTrue(diff.invalidateRoot)
        assertNull(diff.focusTarget)
    }

    @Test fun firstPublishInvalidatesRoot() {
        val diff = diffTrees(null, tree("launcher", node(1)), focusedId = null)
        assertTrue(diff.invalidateRoot)
    }

    @Test fun unchangedTreeInvalidatesNothing() {
        val old = tree("launcher", node(1), node(2, index = 1))
        val new = tree("launcher", node(1), node(2, index = 1))
        val diff = diffTrees(old, new, focusedId = 1)
        assertFalse(diff.invalidateRoot)
        assertTrue(diff.invalidated.isEmpty())
        assertTrue(diff.textChanged.isEmpty())
        assertNull(diff.focusTarget)
    }

    @Test fun orderChangeKeepsFocusAndInvalidatesMovedNodes() {
        val old = tree("launcher", node(1), node(2, index = 1), node(3, index = 2))
        val new = tree("launcher", node(1), node(3, index = 2), node(2, index = 1))
        val diff = diffTrees(old, new, focusedId = 2)
        assertFalse(diff.invalidateRoot)
        assertEquals(setOf(2, 3), diff.invalidated)
        assertNull(diff.focusTarget)
    }

    @Test fun boundsChangeKeepsFocusAndInvalidatesThatNode() {
        val old = tree("launcher", node(1), node(2, index = 1))
        val new = tree("launcher", node(1, top = 40), node(2, index = 1))
        val diff = diffTrees(old, new, focusedId = 1)
        assertFalse(diff.invalidateRoot)
        assertEquals(setOf(1), diff.invalidated)
        assertTrue(diff.textChanged.isEmpty())
        assertNull(diff.focusTarget)
    }

    @Test fun labelOnlyChangeUsesTextInvalidation() {
        val old = tree("launcher", node(1, label = "Old"))
        val new = tree("launcher", node(1, label = "New"))
        val diff = diffTrees(old, new, focusedId = 1)
        assertFalse(diff.invalidateRoot)
        assertTrue(diff.invalidated.isEmpty())
        assertEquals(setOf(1), diff.textChanged)
    }

    @Test fun valueOnlyChangeUsesTextInvalidation() {
        val old = tree("search", node(1, value = "cal"))
        val new = tree("search", node(1, value = "calc"))
        val diff = diffTrees(old, new, focusedId = 1)
        assertTrue(diff.invalidated.isEmpty())
        assertEquals(setOf(1), diff.textChanged)
    }

    @Test fun selectedEnabledAndActionChangesInvalidateTheNode() {
        val old = tree("launcher", node(1))
        val new = tree("launcher", node(1, selected = true, enabled = false, actions = emptyList()))
        val diff = diffTrees(old, new, focusedId = 1)
        assertEquals(setOf(1), diff.invalidated)
        assertTrue(diff.textChanged.isEmpty())
    }

    @Test fun addedAndRemovedNodesAreInvalidated() {
        val old = tree("launcher", node(1), node(2, index = 1))
        val new = tree("launcher", node(1), node(3, index = 1))
        val diff = diffTrees(old, new, focusedId = 1)
        assertFalse(diff.invalidateRoot)
        assertEquals(setOf(3), diff.invalidated)
        assertEquals(setOf(2), diff.removed)
    }

    @Test fun removedNodesAreNeverReportedAsLiveInvalidations() {
        val old = tree("launcher", node(1), node(2, index = 1), node(3, index = 2))
        val new = tree("launcher", node(1))
        val diff = diffTrees(old, new, focusedId = 1)
        assertEquals(setOf(2, 3), diff.removed)
        assertTrue(diff.invalidated.none { it !in new.nodes })
    }

    @Test fun disappearingFocusedNodeMovesToTheNextSurvivor() {
        val old = tree("launcher", node(1), node(2, index = 1), node(3, index = 2))
        val new = tree("launcher", node(1), node(3, index = 1))
        val diff = diffTrees(old, new, focusedId = 2)
        assertFalse(diff.invalidateRoot)
        assertEquals(3, diff.focusTarget)
        assertEquals(setOf(2), diff.removed)
        assertFalse(diff.invalidated.contains(2))
    }

    @Test fun disappearingFocusedNodeFallsBackToThePreviousSurvivor() {
        val old = tree("launcher", node(1), node(2, index = 1), node(3, index = 2))
        val new = tree("launcher", node(1), node(2, index = 1))
        val diff = diffTrees(old, new, focusedId = 3)
        assertEquals(2, diff.focusTarget)
    }

    @Test fun disappearingLastNodeFallsBackToTheHost() {
        val old = tree("launcher", node(1))
        val new = tree("launcher")
        val diff = diffTrees(old, new, focusedId = 1)
        assertTrue(diff.invalidateRoot)
        assertNull(diff.focusTarget)
    }

    @Test fun hostFocusStaysOnTheHostAcrossPublishes() {
        val old = tree("launcher", node(1))
        val new = tree("launcher", node(1, top = 10))
        val diff = diffTrees(old, new, focusedId = null)
        assertFalse(diff.invalidateRoot)
        assertNull(diff.focusTarget)
        assertEquals(setOf(1), diff.invalidated)
    }
}
