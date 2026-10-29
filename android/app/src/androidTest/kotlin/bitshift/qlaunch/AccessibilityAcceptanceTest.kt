package bitshift.qlaunch

import android.app.UiAutomation
import android.os.Bundle
import android.os.ParcelFileDescriptor
import android.os.SystemClock
import android.util.Log
import android.view.accessibility.AccessibilityNodeInfo
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.FixMethodOrder
import org.junit.Test
import org.junit.runner.RunWith
import org.junit.runners.MethodSorters

/// Drives the virtual tree with the same accessibility actions TalkBack sends.
/// `adb shell input` cannot be used: injected touch bypasses the accessibility
/// layer, so it never reaches ExploreByTouchHelper.
/// Run each method in its own `am instrument` invocation: runner cleanup destroys
/// the NativeActivity between tests, and its glue cannot restart in that process.
/// Install with `adb install -r`; connected Gradle tests uninstall the target.
@RunWith(AndroidJUnit4::class)
@FixMethodOrder(MethodSorters.NAME_ASCENDING)
class AccessibilityAcceptanceTest {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val automation: UiAutomation get() = instrumentation.getUiAutomation(
        UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES,
    )

    private fun shell(command: String): String =
        ParcelFileDescriptor.AutoCloseInputStream(automation.executeShellCommand(command))
            .use { it.bufferedReader().readText() }

    private fun windowFocus(): String = shell("dumpsys window")
        .lineSequence().filter { it.contains("mCurrentFocus") }.joinToString("\n")

    private fun launch() {
        // Do not force-stop or start a new instance: the instrumentation runs
        // inside the target process and android-activity supports one
        // NativeActivity instance. Home resumes the existing launcher.
        // Repeated Home on the foreground launcher opens search by design.
        if (!inLauncher() || labels().isEmpty()) shell("input keyevent KEYCODE_HOME")
        SystemClock.sleep(1200)
        requireLauncher()
    }

    private fun root(): AccessibilityNodeInfo? = automation.rootInActiveWindow

    private fun allNodes(): List<AccessibilityNodeInfo> {
        val root = root() ?: return emptyList()
        val nodes = ArrayList<AccessibilityNodeInfo>()
        fun visit(node: AccessibilityNodeInfo) {
            nodes.add(node)
            for (index in 0 until node.childCount) node.getChild(index)?.let(::visit)
        }
        visit(root)
        return nodes
    }

    private fun label(node: AccessibilityNodeInfo): String =
        node.text?.toString()?.takeIf { it.isNotEmpty() }
            ?: node.contentDescription?.toString()?.takeIf { it.isNotEmpty() }
            ?: node.hintText?.toString().orEmpty()

    private fun labels(): List<String> = allNodes().map(::label).filter { it.isNotEmpty() }

    private fun requireLauncher() {
        repeat(20) {
            val current = labels()
            if (current.size >= 4 && current.any { it.contains(" apps") && !it.contains("0 apps") }) {
                return
            }
            SystemClock.sleep(250)
        }
        val active = root()
        throw AssertionError(
            "launcher tree never appeared: active=${active?.packageName}" +
                " children=${active?.childCount} labels=${labels()}",
        )
    }

    private fun matches(node: AccessibilityNodeInfo, text: String): Boolean =
        listOfNotNull(node.text, node.contentDescription, node.hintText)
            .any { it.toString().contains(text) }

    private fun find(text: String): AccessibilityNodeInfo? =
        allNodes().firstOrNull { label(it) == text }
            ?: allNodes().firstOrNull { matches(it, text) }

    private fun require(text: String): AccessibilityNodeInfo {
        val node = find(text)
        assertNotNull("missing node: $text in ${labels()}", node)
        return node!!
    }

    private fun perform(label: String, action: Int, arguments: Bundle? = null): Boolean {
        val node = require(label)
        val result = node.performAction(action, arguments)
        SystemClock.sleep(700)
        return result
    }

    private fun focus(label: String): Boolean {
        val result = require(label).performAction(AccessibilityNodeInfo.ACTION_ACCESSIBILITY_FOCUS)
        SystemClock.sleep(700)
        assertTrue("could not focus $label", result || require(label).isAccessibilityFocused)
        return result
    }

    private fun setText(label: String, text: String): Boolean {
        val arguments = Bundle().apply {
            putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, text)
        }
        return perform(label, AccessibilityNodeInfo.ACTION_SET_TEXT, arguments)
    }

    private fun inLauncher(): Boolean =
        root()?.packageName?.toString() == "bitshift.qlaunch"

    private fun goHome() {
        find("Close search")?.let {
            it.performAction(AccessibilityNodeInfo.ACTION_CLICK)
            SystemClock.sleep(700)
        }
        repeat(4) {
            if (find("Home, search apps") != null) return
            val step = find("Previous page, Home") ?: find("Next page, Home")
            assertNotNull("no Home tab in ${labels()}", step)
            step!!.performAction(AccessibilityNodeInfo.ACTION_CLICK)
            SystemClock.sleep(900)
        }
        assertNotNull("could not reach Home: ${labels()}", find("Home, search apps"))
    }

    private fun goApps() {
        goHome()
        val step = find("Previous page, Apps") ?: find("Next page, Apps")
        assertNotNull("no Apps tab in ${labels()}", step)
        step!!.performAction(AccessibilityNodeInfo.ACTION_CLICK)
        SystemClock.sleep(900)
        assertNotNull("could not reach Apps: ${labels()}", find("Apps, search apps"))
    }

    @Test
    fun a_tabsGridAndAppsArePublishedInOrder() {
        launch()
        goHome()
        val published = labels()
        Log.i(TAG, "Home tree: $published")
        val tabs = published.indexOfFirst { it.contains("search apps") }
        val grid = published.indexOfFirst { it.contains("page") && it.contains("apps") }
        val firstApp = published.indexOfFirst { it == "Firefox" }
        assertTrue("tab before grid in $published", tabs >= 0 && tabs < grid)
        assertTrue("grid before apps in $published", grid >= 0 && grid < firstApp)
        assertTrue("five Home apps", listOf("Firefox", "Jellyfin", "LingoAce", "VLC", "WuKong")
            .all { published.contains(it) })
    }

    @Test
    fun b_clickLaunchesAndLongClickOpensMenus() {
        launch()
        goHome()
        assertTrue(perform("Firefox", AccessibilityNodeInfo.ACTION_CLICK))
        SystemClock.sleep(1200)
        val focus = windowFocus()
        assertTrue("Firefox did not launch: $focus", focus.contains("org.mozilla.firefox"))

        launch()
        goHome()
        assertTrue(perform("Firefox", AccessibilityNodeInfo.ACTION_LONG_CLICK))
        val appMenu = labels()
        Log.i(TAG, "App menu: $appMenu")
        assertTrue("app menu: $appMenu", listOf("Rename", "Move", "Uninstall").all { appMenu.contains(it) })
        val dismiss = find("Cancel") ?: find("Back")
        assertNotNull("no dismiss action in $appMenu", dismiss)
        dismiss!!.performAction(AccessibilityNodeInfo.ACTION_CLICK)
        SystemClock.sleep(700)
        assertTrue("menu survived dismiss: ${labels()}", find("Rename") == null)

        launch()
        goHome()
        val grid = require("Home, page")
        assertTrue(grid.performAction(AccessibilityNodeInfo.ACTION_LONG_CLICK))
        SystemClock.sleep(700)
        val launcherMenu = labels()
        Log.i(TAG, "Launcher menu: $launcherMenu")
        assertTrue("launcher menu: $launcherMenu",
            launcherMenu.any { it.contains("Change wallpaper") || it.contains("Wallpaper") })
        (find("Cancel") ?: find("Close") ?: find("Back"))!!.performAction(
            AccessibilityNodeInfo.ACTION_CLICK
        )
    }

    @Test
    fun c_searchShowsKeyboardAndKeepsFocusWhileTyping() {
        launch()
        goHome()
        assertTrue(perform("search apps", AccessibilityNodeInfo.ACTION_CLICK))
        assertNotNull("search field missing: ${labels()}", find("Search apps"))
        val inputMethod = shell("dumpsys input_method")
            .lineSequence().filter { it.contains("mInputShown") || it.contains("mIsInputViewShown") }
            .joinToString("\n")
        Log.i(TAG, "IME: $inputMethod")
        assertTrue("keyboard did not open: $inputMethod", inputMethod.contains("mInputShown=true"))

        focus("Search apps")
        assertTrue("initial search focus missing", require("Search apps").isAccessibilityFocused)
        for (query in listOf("c", "ca", "cal", "calc")) {
            assertTrue(setText("Search apps", query))
            SystemClock.sleep(700)
            val field = require("Search apps")
            assertTrue("query was not applied: ${field.text}", field.text?.toString() == query)
            assertTrue("focus lost after $query: field=$field focused=" +
                automation.findFocus(AccessibilityNodeInfo.FOCUS_ACCESSIBILITY), field.isAccessibilityFocused)
        }
        val results = labels()
        Log.i(TAG, "Search results: $results")
        assertTrue("Calculator result missing: $results", results.any { it.contains("Calculator") })

        assertTrue(perform("Close search", AccessibilityNodeInfo.ACTION_CLICK))
        assertNotNull("launcher did not return: ${labels()}", find("page"))
    }

    @Test
    fun d_scrollAndPageStepsPreserveFocusAndHideOldNodes() {
        launch()
        goApps()
        focus("Gmail")
        val grid = require("Apps, page")
        assertTrue(grid.performAction(AccessibilityNodeInfo.ACTION_SCROLL_FORWARD))
        SystemClock.sleep(1000)
        val afterScroll = require("Gmail")
        assertTrue("Gmail focus lost after scroll", afterScroll.isAccessibilityFocused)
        val scrolled = labels()
        Log.i(TAG, "Scrolled Apps: $scrolled")
        assertTrue("second page of apps did not appear", scrolled.any { it.contains("Termius") })

        focus("Gmail")
        val tab = require("Next page")
        assertTrue(tab.performAction(AccessibilityNodeInfo.ACTION_CLICK))
        SystemClock.sleep(1200)
        val nextPage = labels()
        Log.i(TAG, "Next page: $nextPage")
        assertFalse("old page node still published: $nextPage", nextPage.contains("Gmail"))
        assertTrue("page did not change: $nextPage", nextPage.any { it.contains("Home, page 2 of 3") })
        assertTrue("focus lost on page step", allNodes().any { it.isAccessibilityFocused })
    }

    @Test
    fun e_itemActionDuringPageMotionIsIgnored() {
        launch()
        goHome()
        val firefox = require("Firefox")
        val tab = require("Previous page, Apps")
        assertTrue(tab.performAction(AccessibilityNodeInfo.ACTION_CLICK))
        // The pager is now animating and the old frame is still published.
        firefox.performAction(AccessibilityNodeInfo.ACTION_CLICK)
        SystemClock.sleep(1500)
        val focus = windowFocus()
        assertTrue("item action during motion launched Firefox: $focus", focus.contains("bitshift.qlaunch"))
    }

    private companion object {
        const val TAG = "AccessibilityAcceptance"
    }
}
