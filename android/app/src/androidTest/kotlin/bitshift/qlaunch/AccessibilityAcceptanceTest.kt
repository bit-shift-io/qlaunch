package bitshift.qlaunch

import android.app.UiAutomation
import android.accessibilityservice.AccessibilityServiceInfo
import android.graphics.Bitmap
import android.graphics.Rect
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
import org.junit.Assume.assumeTrue
import org.junit.FixMethodOrder
import org.junit.Test
import org.junit.runner.RunWith
import org.junit.runners.MethodSorters
import java.io.File

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

    private fun capture(name: String) {
        val directory = File(instrumentation.targetContext.getExternalFilesDir(null), "task6")
        directory.mkdirs()
        File(directory, "$name.png").outputStream().use {
            check(automation.takeScreenshot().compress(Bitmap.CompressFormat.PNG, 100, it))
        }
        File(directory, "$name.txt").writeText(allNodes().map { node ->
            val bounds = Rect()
            node.getBoundsInScreen(bounds)
            "${label(node)}: $bounds"
        }.joinToString("\n"))
    }

    private fun tapNode(text: String) {
        val bounds = Rect()
        require(text).getBoundsInScreen(bounds)
        shell("input tap ${bounds.centerX()} ${bounds.centerY()}")
        SystemClock.sleep(700)
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
    fun e_oldItemActionAfterPageStepDoesNotLaunch() {
        launch()
        goHome()
        val firefox = require("Firefox")
        val tab = require("Previous page, Apps")
        assertTrue(tab.performAction(AccessibilityNodeInfo.ACTION_CLICK))
        // Send immediately after the page step; device scheduling is not
        // deterministic, so rejection may also be due to a stale frame.
        firefox.performAction(AccessibilityNodeInfo.ACTION_CLICK)
        SystemClock.sleep(1500)
        val focus = windowFocus()
        assertTrue("old item action launched Firefox: $focus", inLauncher())
        assertNotNull("page step did not reach Apps: ${labels()}", find("Apps, page 1 of 3"))
        // This device-level race is a smoke check, not proof of the motion
        // gate. Rust's deterministic test proves the same valid request is
        // rejected during settle and accepted once motion is cancelled.
    }

    @Test
    fun f_menuSubflowsAndBounds() {
        launch()
        goHome()
        focus("Firefox")
        capture("home")
        assertTrue(perform("Firefox", AccessibilityNodeInfo.ACTION_LONG_CLICK))
        assertTrue("screen change left focus on a hidden node",
            allNodes().any { it.isAccessibilityFocused })
        focus("Rename")
        capture("app-menu")
        assertTrue(perform("Rename", AccessibilityNodeInfo.ACTION_CLICK))
        assertNotNull("rename field missing: ${labels()}", find("App name"))
        assertTrue(perform("App name", AccessibilityNodeInfo.ACTION_CLICK))
        capture("rename")
        assertTrue("rename keyboard did not open", shell("dumpsys input_method").contains("mInputShown=true"))
        assertTrue(perform("Back", AccessibilityNodeInfo.ACTION_CLICK))
        assertTrue(perform("Move", AccessibilityNodeInfo.ACTION_CLICK))
        assertTrue("move destinations missing: ${labels()}", labels().any { it == "Games" })
        capture("move")
        assertTrue(perform("Back", AccessibilityNodeInfo.ACTION_CLICK))
        assertTrue(perform("Uninstall", AccessibilityNodeInfo.ACTION_CLICK))
        assertTrue("system uninstall confirmation missing: ${labels()}",
            root()?.packageName?.toString() != "bitshift.qlaunch" && find("Firefox") != null)
        // Only verify and cancel the system confirmation. Never uninstall an app.
        capture("uninstall-confirmation")
        assertTrue(perform("Cancel", AccessibilityNodeInfo.ACTION_CLICK))
        launch()
        if (find("Cancel") != null) perform("Cancel", AccessibilityNodeInfo.ACTION_CLICK)
        goHome()
        assertTrue(perform("search apps", AccessibilityNodeInfo.ACTION_CLICK))
        assertTrue(setText("Search apps", "calc"))
        SystemClock.sleep(700)
        assertNotNull("Calculator missing: ${labels()}", find("Calculator"))
        focus("Calculator")
        capture("search-result")
        assertTrue(perform("Close search", AccessibilityNodeInfo.ACTION_CLICK))
    }

    // Requires an external private-store snapshot and restoration after the run.
    @Test
    fun g_temporaryFolderNavigationAndBounds() {
        assumeTrue("requires external store backup/restore",
            InstrumentationRegistry.getArguments().getString("allowTemporaryFolder") == "true")
        launch()
        goHome()
        assertTrue(perform("Home, page", AccessibilityNodeInfo.ACTION_LONG_CLICK))
        assertTrue(perform("Create Folder", AccessibilityNodeInfo.ACTION_CLICK))
        assertTrue(setText("Folder name", "Task6 temporary folder"))
        assertTrue(perform("Create", AccessibilityNodeInfo.ACTION_CLICK))
        assertTrue(perform("Task6 temporary folder", AccessibilityNodeInfo.ACTION_CLICK))
        focus("Task6 temporary folder")
        capture("folder")
        assertNotNull("folder back missing: ${labels()}", find("Back"))
        assertTrue(perform("Back", AccessibilityNodeInfo.ACTION_CLICK))
        assertNotNull("launcher missing: ${labels()}", find("Home, search apps"))
        assertFalse("closed folder back remains: ${labels()}", labels().contains("Back"))
    }

    // Run with TalkBack disabled externally, restoring its settings afterwards.
    @Test
    fun h_ordinaryTouchAndKeyboardWithTalkBackOff() {
        assertFalse("disable TalkBack before this check",
            shell("settings get secure enabled_accessibility_services").contains("talkback"))
        launch()
        goHome()
        tapNode("Firefox")
        assertTrue("ordinary tap did not launch", windowFocus().contains("org.mozilla.firefox"))
        launch()
        goHome()
        val bounds = Rect()
        require("Firefox").getBoundsInScreen(bounds)
        shell("input swipe ${bounds.centerX()} ${bounds.centerY()} ${bounds.centerX()} ${bounds.centerY()} 700")
        SystemClock.sleep(700)
        assertNotNull("ordinary long press failed: ${labels()}", find("Rename"))
        tapNode("Rename")
        tapNode("App name")
        assertTrue("ordinary rename keyboard did not open",
            shell("dumpsys input_method").contains("mInputShown=true"))
        capture("ordinary-rename-ime")
        shell("input keyevent KEYCODE_Z")
        SystemClock.sleep(700)
        assertTrue("rename did not accept key event: ${labels()}", require("App name").text.toString().contains("z"))
        // Cancel rather than saving the temporary edit.
        tapNode("Back")
        tapNode("Cancel")
        tapNode("Home, search apps")
        assertTrue("ordinary search keyboard did not open",
            shell("dumpsys input_method").contains("mInputShown=true"))
        shell("input text calc")
        SystemClock.sleep(1000)
        assertTrue("search did not accept hardware key events", require("Search apps").text.toString() == "calc")
        assertNotNull("ordinary search result missing", find("Calculator"))
        capture("ordinary-search-ime")
        // Tablet's on-screen Gboard C key (from the captured 1200x1920 layout).
        shell("input tap 485 1650")
        SystemClock.sleep(700)
        assertTrue("soft keyboard tap did not update query", require("Search apps").text.toString() == "calcc")
        tapNode("Close search")
        // Pull down from blank content, not from the system status bar.
        shell("input swipe 600 1000 600 1450 400")
        SystemClock.sleep(1000)
        val destination = root()?.packageName?.toString()
        assertTrue("pull-down did not reach notifications or its permission gate: ${windowFocus()}",
            destination == "com.android.systemui" || destination == "com.android.settings")
        Log.i(TAG, "Pull-down destination: $destination (Settings means ScreenLockService is disabled)")
        shell("input keyevent KEYCODE_BACK")
        SystemClock.sleep(700)
        goHome()
        // With ScreenLockService absent this opens its permission screen rather
        // than locking the user's device; never run this with that service on.
        assertFalse(shell("settings get secure enabled_accessibility_services").contains("ScreenLockService"))
        shell("input tap 600 1000")
        shell("input tap 600 1000")
        SystemClock.sleep(1000)
        assertTrue("blank double tap did not reach lock permission gate",
            root()?.packageName?.toString() == "com.android.settings")
        shell("input keyevent KEYCODE_BACK")
    }

    @Test
    fun i_nodesAreExposedWithoutIncludingUnimportantViews() {
        val original = automation.serviceInfo
        val flags = original.flags
        original.flags = flags and AccessibilityServiceInfo.FLAG_INCLUDE_NOT_IMPORTANT_VIEWS.inv()
        automation.serviceInfo = original
        try {
            launch()
            goHome()
            val node = require("Firefox")
            assertTrue("Firefox is not important for accessibility: $node", node.isImportantForAccessibility)
        } finally {
            val restored = automation.serviceInfo
            restored.flags = flags
            automation.serviceInfo = restored
        }
    }

    private companion object {
        const val TAG = "AccessibilityAcceptance"
    }
}
