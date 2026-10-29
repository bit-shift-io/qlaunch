package bitshift.qlaunch

import android.content.Context
import android.graphics.Rect
import android.os.Bundle
import android.view.MotionEvent
import android.view.View
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import androidx.core.view.ViewCompat
import androidx.core.view.accessibility.AccessibilityNodeInfoCompat
import androidx.customview.widget.ExploreByTouchHelper
import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.roundToInt

/// Transparent host for the virtual tree Rust publishes. It never consumes
/// ordinary touches and never takes input focus: the iced surface and IME keep
/// those. TalkBack only sees the virtual nodes.
class LauncherAccessibilityView(context: Context) : View(context) {
    var onAction: ((nodeId: Int, action: String, value: String, revision: Long) -> Unit)? = null

    private var revision = 0L
    private var tree: TreeSnapshot? = null
    private var parsed: ParsedPayload? = null
    private val helper = Helper()

    init {
        importantForAccessibility = IMPORTANT_FOR_ACCESSIBILITY_YES
        isClickable = false
        ViewCompat.setAccessibilityDelegate(this, helper)
        // ExploreByTouchHelper makes its host focusable. Keyboard focus belongs
        // to the iced IME view, so clear it again.
        isFocusable = false
        isFocusableInTouchMode = false
    }

    fun publish(json: String) {
        val payload = parse(json) ?: return
        parsed = payload
        apply(payload, announce = true)
    }

    override fun onSizeChanged(width: Int, height: Int, oldWidth: Int, oldHeight: Int) {
        super.onSizeChanged(width, height, oldWidth, oldHeight)
        // The first publish can arrive before layout; rescale once the real
        // view size is known. Do not repeat the announcement.
        parsed?.let { apply(it, announce = false) }
    }

    override fun dispatchHoverEvent(event: MotionEvent): Boolean =
        helper.dispatchHoverEvent(event) || super.dispatchHoverEvent(event)

    private fun apply(payload: ParsedPayload, announce: Boolean) {
        val next = scale(payload)
        val focused = helper.accessibilityFocusedVirtualViewId
            .takeUnless { it == ExploreByTouchHelper.HOST_ID || it == ExploreByTouchHelper.INVALID_ID }
        val diff = diffTrees(tree, next, focused)
        // Clear while the old tree still exists: clearing afterwards can ask
        // the provider to populate a node which has already been removed.
        if (focused != null && focused !in next.nodes) {
            helper.getAccessibilityNodeProvider(this)?.performAction(
                focused,
                AccessibilityNodeInfo.ACTION_CLEAR_ACCESSIBILITY_FOCUS,
                null,
            )
        }
        revision = payload.revision
        tree = next

        if (diff.invalidateRoot) {
            helper.invalidateRoot()
        }
        // ExploreByTouchHelper repopulates a virtual view on invalidation, so
        // never invalidate an id that the new tree no longer contains.
        for (id in diff.invalidated) {
            if (id in next.nodes) helper.invalidateVirtualView(id)
        }
        for (id in diff.textChanged) {
            if (id in next.nodes) {
                helper.invalidateVirtualView(id, AccessibilityEvent.CONTENT_CHANGE_TYPE_TEXT)
            }
        }
        if (diff.removed.isNotEmpty()) {
            helper.sendEventForVirtualView(
                ExploreByTouchHelper.HOST_ID,
                AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED,
            )
        }
        diff.focusTarget?.let {
            helper.getAccessibilityNodeProvider(this)?.performAction(
                it,
                AccessibilityNodeInfo.ACTION_ACCESSIBILITY_FOCUS,
                null,
            )
        }
        if (announce) payload.announcement?.let(::announceForAccessibility)
    }

    private fun scale(payload: ParsedPayload): TreeSnapshot {
        val scaleX = if (width > 0 && payload.windowWidth > 0f) width / payload.windowWidth else 1f
        val scaleY = if (height > 0 && payload.windowHeight > 0f) height / payload.windowHeight else 1f
        val nodes = payload.nodes.mapValues { (_, node) ->
            NodeSnapshot(
                node.id,
                node.role,
                NodeRect(
                    (node.left * scaleX).roundToInt(),
                    (node.top * scaleY).roundToInt(),
                    (node.right * scaleX).roundToInt(),
                    (node.bottom * scaleY).roundToInt(),
                ),
                node.label,
                node.value,
                node.selected,
                node.enabled,
                node.actions,
            )
        }
        return TreeSnapshot(payload.screen, payload.order, nodes)
    }

    private fun parse(json: String): ParsedPayload? = runCatching {
        val data = JSONObject(json)
        val window = data.optJSONObject("window")
        val nodes = LinkedHashMap<Int, ParsedNode>()
        val order = ArrayList<Int>()
        val array: JSONArray = data.getJSONArray("nodes")
        for (index in 0 until array.length()) {
            val node = array.getJSONObject(index)
            val id = node.getInt("id")
            val bounds = node.getJSONArray("bounds")
            val left = bounds.getDouble(0).toFloat()
            val top = bounds.getDouble(1).toFloat()
            val nodeWidth = bounds.getDouble(2).toFloat()
            val nodeHeight = bounds.getDouble(3).toFloat()
            val actions = ArrayList<String>()
            node.optJSONArray("actions")?.let { list ->
                for (position in 0 until list.length()) actions.add(list.getString(position))
            }
            nodes[id] = ParsedNode(
                id,
                node.getString("role"),
                left,
                top,
                left + nodeWidth,
                top + nodeHeight,
                node.optString("label"),
                if (node.isNull("value")) null else node.getString("value"),
                node.optBoolean("selected"),
                node.optBoolean("enabled", true),
                actions,
            )
            order.add(id)
        }
        ParsedPayload(
            data.getLong("revision"),
            data.getString("screen"),
            if (data.isNull("announcement")) null else data.getString("announcement"),
            window?.optDouble("width", 0.0)?.toFloat() ?: 0f,
            window?.optDouble("height", 0.0)?.toFloat() ?: 0f,
            order,
            nodes,
        )
    }.getOrNull()

    private data class ParsedNode(
        val id: Int,
        val role: String,
        val left: Float,
        val top: Float,
        val right: Float,
        val bottom: Float,
        val label: String,
        val value: String?,
        val selected: Boolean,
        val enabled: Boolean,
        val actions: List<String>,
    )

    private data class ParsedPayload(
        val revision: Long,
        val screen: String,
        val announcement: String?,
        val windowWidth: Float,
        val windowHeight: Float,
        val order: List<Int>,
        val nodes: Map<Int, ParsedNode>,
    )

    private inner class Helper : ExploreByTouchHelper(this) {
        override fun getVirtualViewAt(x: Float, y: Float): Int {
            val current = tree ?: return HOST_ID
            var scroll: Int? = null
            for (id in current.order) {
                val node = current.nodes[id] ?: continue
                if (node.bounds.left >= node.bounds.right || node.bounds.top >= node.bounds.bottom) continue
                if (x < node.bounds.left || x >= node.bounds.right ||
                    y < node.bounds.top || y >= node.bounds.bottom) continue
                // Item rows and other specific nodes sit on top of the scroll
                // container's bounds.
                if (node.role == "scroll") {
                    if (scroll == null) scroll = id
                } else {
                    return id
                }
            }
            return scroll ?: HOST_ID
        }

        override fun getVisibleVirtualViews(ids: MutableList<Int>) {
            val current = tree ?: return
            for (id in current.order) {
                val node = current.nodes[id] ?: continue
                if (node.bounds.left < node.bounds.right && node.bounds.top < node.bounds.bottom) {
                    ids.add(id)
                }
            }
        }

        override fun onPopulateNodeForVirtualView(id: Int, node: AccessibilityNodeInfoCompat) {
            val snapshot = tree?.nodes?.get(id)
            if (snapshot == null) {
                // A publish can race a late platform query. Populate a harmless
                // placeholder instead of letting ExploreByTouchHelper crash.
                node.text = "QLaunch"
                node.className = "android.view.View"
                node.isImportantForAccessibility = true
                node.isVisibleToUser = false
                node.setBoundsInParent(Rect())
                return
            }
            node.text = when (snapshot.role) {
                "text_field" -> snapshot.value ?: ""
                else -> snapshot.label.ifEmpty { "QLaunch" }
            }
            node.className = className(snapshot.role)
            // Provider nodes are not marked important for the platform the way
            // real views are. Without this, services that do not request
            // unimportant views never see the launcher's virtual tree.
            node.isImportantForAccessibility = true
            if (snapshot.role == "header") node.isHeading = true
            if (snapshot.role == "text_field") node.hintText = snapshot.label
            node.contentDescription = snapshot.label.takeIf {
                it.isNotEmpty() && (snapshot.role != "text_field" || snapshot.value.isNullOrEmpty())
            }
            node.setBoundsInParent(
                Rect(snapshot.bounds.left, snapshot.bounds.top, snapshot.bounds.right, snapshot.bounds.bottom)
            )
            node.isVisibleToUser = true
            node.isEnabled = snapshot.enabled
            node.isSelected = snapshot.selected
            node.isEditable = "set_text" in snapshot.actions
            node.isClickable = "click" in snapshot.actions
            node.isLongClickable = "long_click" in snapshot.actions
            node.isScrollable = snapshot.actions.any {
                it == "scroll_forward" || it == "scroll_backward" ||
                    it == "scroll_left" || it == "scroll_right"
            }
            for (action in snapshot.actions) {
                when (action) {
                    "click" -> node.addAction(AccessibilityNodeInfoCompat.ACTION_CLICK)
                    "long_click" -> node.addAction(AccessibilityNodeInfoCompat.ACTION_LONG_CLICK)
                    "scroll_forward" -> node.addAction(AccessibilityNodeInfoCompat.ACTION_SCROLL_FORWARD)
                    "scroll_backward" -> node.addAction(AccessibilityNodeInfoCompat.ACTION_SCROLL_BACKWARD)
                    "scroll_left" -> node.addAction(
                        AccessibilityNodeInfoCompat.AccessibilityActionCompat.ACTION_SCROLL_LEFT
                    )
                    "scroll_right" -> node.addAction(
                        AccessibilityNodeInfoCompat.AccessibilityActionCompat.ACTION_SCROLL_RIGHT
                    )
                    "set_text" -> node.addAction(AccessibilityNodeInfoCompat.ACTION_SET_TEXT)
                    "dismiss" -> node.addAction(AccessibilityNodeInfoCompat.ACTION_DISMISS)
                }
            }
        }

        override fun onPerformActionForVirtualView(id: Int, action: Int, arguments: Bundle?): Boolean {
            val snapshot = tree?.nodes?.get(id) ?: return false
            val name = when (action) {
                AccessibilityNodeInfo.ACTION_CLICK -> "click"
                AccessibilityNodeInfo.ACTION_LONG_CLICK -> "long_click"
                AccessibilityNodeInfo.ACTION_SCROLL_FORWARD -> "scroll_forward"
                AccessibilityNodeInfo.ACTION_SCROLL_BACKWARD -> "scroll_backward"
                AccessibilityNodeInfoCompat.AccessibilityActionCompat.ACTION_SCROLL_LEFT.id -> "scroll_left"
                AccessibilityNodeInfoCompat.AccessibilityActionCompat.ACTION_SCROLL_RIGHT.id -> "scroll_right"
                AccessibilityNodeInfo.ACTION_SET_TEXT -> "set_text"
                AccessibilityNodeInfo.ACTION_DISMISS -> "dismiss"
                else -> return false
            }
            if (name !in snapshot.actions) return false
            val value = if (action == AccessibilityNodeInfo.ACTION_SET_TEXT) {
                arguments?.getCharSequence(
                    AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE
                )?.toString().orEmpty()
            } else {
                ""
            }
            onAction?.invoke(id, name, value, revision)
            return true
        }
    }

    private fun className(role: String): CharSequence = when (role) {
        "button" -> "android.widget.Button"
        "tab" -> "android.widget.TabWidget"
        "header", "text" -> "android.widget.TextView"
        "text_field" -> "android.widget.EditText"
        "scroll" -> "android.widget.ScrollView"
        else -> "android.view.View"
    }
}
