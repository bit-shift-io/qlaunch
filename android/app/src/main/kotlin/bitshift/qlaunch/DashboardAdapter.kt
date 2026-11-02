package bitshift.qlaunch

import android.Manifest
import android.app.Activity
import android.app.AlarmManager
import android.content.ComponentName
import android.content.ContentUris
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.provider.AlarmClock
import android.provider.CalendarContract
import android.text.format.DateFormat
import org.json.JSONArray
import org.json.JSONObject
import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.HttpURLConnection
import java.net.URL
import java.net.URLEncoder
import java.text.SimpleDateFormat
import java.util.Calendar
import java.util.Date
import java.util.Locale
import java.util.concurrent.Executors

object DashboardAdapter {
    private val executor = Executors.newSingleThreadExecutor()

    // In-memory weather cache: cache key -> (timestamp, json)
    private var cachedWeatherLocation: String? = null
    private var cachedWeatherJson: String? = null
    private var cachedWeatherTime: Long = 0
    private const val WEATHER_CACHE_TTL_MS = 30 * 60 * 1000L // 30 minutes

    // --- CLOCK & DATE ---
    fun getCurrentTimeJson(context: Context): String {
        val now = Date()
        val is24 = DateFormat.is24HourFormat(context)
        val timeFormat = if (is24) SimpleDateFormat("HH:mm", Locale.getDefault()) else SimpleDateFormat("h:mm a", Locale.getDefault())
        val dateFormat = SimpleDateFormat("EEE, d MMM", Locale.getDefault())
        return JSONObject().apply {
            put("time", timeFormat.format(now))
            put("date", dateFormat.format(now))
        }.toString()
    }

    // --- ALARM ---
    fun getNextAlarmJson(context: Context): String {
        val alarmManager = context.getSystemService(Context.ALARM_SERVICE) as? AlarmManager
            ?: return "{}"
        val nextAlarm = alarmManager.nextAlarmClock ?: return "{}"
        val triggerTime = nextAlarm.triggerTime
        val is24 = DateFormat.is24HourFormat(context)
        val format = if (is24) SimpleDateFormat("HH:mm", Locale.getDefault()) else SimpleDateFormat("h:mm a", Locale.getDefault())
        val timeStr = format.format(Date(triggerTime))

        return JSONObject().apply {
            put("has_alarm", true)
            put("trigger_time", triggerTime)
            put("time_str", timeStr)
        }.toString()
    }

    // --- CALENDAR ---
    fun hasCalendarPermission(context: Context): Boolean {
        return context.checkSelfPermission(Manifest.permission.READ_CALENDAR) == PackageManager.PERMISSION_GRANTED
    }

    fun requestCalendarPermission(activity: Activity) {
        activity.requestPermissions(arrayOf(Manifest.permission.READ_CALENDAR), 2002)
    }

    fun getCalendarEventsJson(context: Context): String {
        val result = JSONObject()
        if (!hasCalendarPermission(context)) {
            result.put("permission_granted", false)
            result.put("events", JSONArray())
            return result.toString()
        }
        result.put("permission_granted", true)

        val now = System.currentTimeMillis()
        val cal = Calendar.getInstance()
        cal.timeInMillis = now
        cal.add(Calendar.DAY_OF_YEAR, 2)
        cal.set(Calendar.HOUR_OF_DAY, 0)
        cal.set(Calendar.MINUTE, 0)
        cal.set(Calendar.SECOND, 0)
        cal.set(Calendar.MILLISECOND, 0)
        val endOfTomorrow = cal.timeInMillis

        val builder = CalendarContract.Instances.CONTENT_URI.buildUpon()
        ContentUris.appendId(builder, now)
        ContentUris.appendId(builder, endOfTomorrow)

        val projection = arrayOf(
            CalendarContract.Instances.EVENT_ID,
            CalendarContract.Instances.TITLE,
            CalendarContract.Instances.BEGIN,
            CalendarContract.Instances.END,
            CalendarContract.Instances.ALL_DAY,
            CalendarContract.Instances.EVENT_COLOR,
            CalendarContract.Instances.CALENDAR_COLOR
        )

        val eventsArray = JSONArray()
        val is24 = DateFormat.is24HourFormat(context)
        val timeFormat = if (is24) SimpleDateFormat("HH:mm", Locale.getDefault()) else SimpleDateFormat("h:mm a", Locale.getDefault())

        try {
            context.contentResolver.query(
                builder.build(),
                projection,
                null,
                null,
                "${CalendarContract.Instances.BEGIN} ASC"
            )?.use { cursor ->
                val idIdx = cursor.getColumnIndex(CalendarContract.Instances.EVENT_ID)
                val titleIdx = cursor.getColumnIndex(CalendarContract.Instances.TITLE)
                val beginIdx = cursor.getColumnIndex(CalendarContract.Instances.BEGIN)
                val endIdx = cursor.getColumnIndex(CalendarContract.Instances.END)
                val allDayIdx = cursor.getColumnIndex(CalendarContract.Instances.ALL_DAY)
                val eventColorIdx = cursor.getColumnIndex(CalendarContract.Instances.EVENT_COLOR)
                val calColorIdx = cursor.getColumnIndex(CalendarContract.Instances.CALENDAR_COLOR)

                while (cursor.moveToNext() && eventsArray.length() < 25) {
                    val eventId = cursor.getLong(idIdx)
                    val title = cursor.getString(titleIdx) ?: "(No title)"
                    val begin = cursor.getLong(beginIdx)
                    val end = cursor.getLong(endIdx)
                    val allDay = cursor.getInt(allDayIdx) != 0

                    var color = if (eventColorIdx >= 0 && !cursor.isNull(eventColorIdx)) {
                        cursor.getInt(eventColorIdx)
                    } else if (calColorIdx >= 0 && !cursor.isNull(calColorIdx)) {
                        cursor.getInt(calColorIdx)
                    } else {
                        0xFF4285F4.toInt() // Default blue
                    }
                    // Ensure opacity
                    color = color or (0xFF shl 24)

                    // Calendar rows show only the start time (or "All day").
                    // The end time is deliberately never rendered.
                    val timeStr = if (allDay) "All day" else timeFormat.format(Date(begin))

                    val eventObj = JSONObject().apply {
                        put("event_id", eventId)
                        put("title", title)
                        put("begin", begin)
                        put("end", end)
                        put("all_day", allDay)
                        put("time_str", timeStr)
                        put("color", color)
                    }
                    eventsArray.put(eventObj)
                }
            }
        } catch (e: Exception) {
            android.util.Log.w("qlaunch", "Error querying calendar instances", e)
        }

        result.put("events", eventsArray)
        return result.toString()
    }

    // --- WEATHER ---
    fun fetchWeatherJsonSync(location: String): String {
        val trimmed = location.trim()
        if (trimmed.isEmpty()) {
            return JSONObject().apply {
                put("error", "No location configured")
            }.toString()
        }

        val now = System.currentTimeMillis()
        if (trimmed == cachedWeatherLocation && cachedWeatherJson != null && (now - cachedWeatherTime < WEATHER_CACHE_TTL_MS)) {
            return cachedWeatherJson!!
        }

        try {
            // Step 1: Geocoding lookup
            // Open-Meteo's geocoder takes `countryCode`; `country` was ignored,
            // which let a postcode like 5000 resolve to Zaragoza, Spain.
            val geoUrl = "https://geocoding-api.open-meteo.com/v1/search?name=${URLEncoder.encode(trimmed, "UTF-8")}&count=1&countryCode=AU&language=en&format=json"
            val geoResp = httpGet(geoUrl)
            val geoJson = JSONObject(geoResp)
            val results = geoJson.optJSONArray("results")
            if (results == null || results.length() == 0) {
                return JSONObject().apply {
                    put("error", "Location not found in Australia: $trimmed")
                }.toString()
            }
            val first = results.getJSONObject(0)
            val lat = first.getDouble("latitude")
            val lon = first.getDouble("longitude")
            val name = first.optString("name", trimmed)

            // Step 2: Forecast lookup. BOM ACCESS is requested first, but it
            // currently returns nulls for Australian locations, so fall back
            // to the default model rather than rendering zeros.
            val baseForecast =
                "https://api.open-meteo.com/v1/forecast?latitude=$lat&longitude=$lon&current=temperature_2m,weather_code&daily=weather_code,temperature_2m_max,temperature_2m_min&timezone=auto"
            var fJson = JSONObject(httpGet("$baseForecast&models=bom_access_global"))
            var current = fJson.optJSONObject("current")
            var missing = current == null || current.isNull("temperature_2m")
            if (missing) {
                fJson = JSONObject(httpGet(baseForecast))
                current = fJson.optJSONObject("current")
                missing = current == null || current.isNull("temperature_2m")
            }
            if (missing || current == null) {
                return JSONObject().apply {
                    put("error", "Weather unavailable for $trimmed")
                }.toString()
            }

            val daily = fJson.optJSONObject("daily")

            val currentTemp = current.optDouble("temperature_2m", 0.0)
            val weatherCode = current.optInt("weather_code", 0)

            val maxTemp = daily?.optJSONArray("temperature_2m_max")?.optDouble(0, currentTemp) ?: currentTemp
            val minTemp = daily?.optJSONArray("temperature_2m_min")?.optDouble(0, currentTemp) ?: currentTemp

            val out = JSONObject().apply {
                put("location_label", name)
                put("current_temp", currentTemp)
                put("temp_min", minTemp)
                put("temp_max", maxTemp)
                put("condition_code", weatherCode)
            }.toString()

            cachedWeatherLocation = trimmed
            cachedWeatherJson = out
            cachedWeatherTime = now
            return out
        } catch (e: Exception) {
            android.util.Log.e("qlaunch", "Weather fetch failed", e)
            return JSONObject().apply {
                put("error", e.message ?: "Weather request failed")
            }.toString()
        }
    }

    private fun httpGet(urlStr: String): String {
        val url = URL(urlStr)
        val conn = url.openConnection() as HttpURLConnection
        conn.requestMethod = "GET"
        conn.connectTimeout = 10000
        conn.readTimeout = 12000
        conn.setRequestProperty("User-Agent", "QLaunch/1.0 (Android)")
        conn.instanceFollowRedirects = true

        val code = conn.responseCode
        if (code != HttpURLConnection.HTTP_OK) {
            throw RuntimeException("HTTP $code from $urlStr")
        }

        conn.inputStream.use { stream ->
            BufferedReader(InputStreamReader(stream, Charsets.UTF_8)).use { reader ->
                return reader.readText()
            }
        }
    }

    // --- APP DISCOVERY & LAUNCH ---
    fun getCategoryAppsJson(context: Context, category: String): String {
        val pm = context.packageManager
        val recommendedPackages = mutableSetOf<String>()

        when (category.lowercase()) {
            "clock" -> {
                val alarmIntent = Intent(AlarmClock.ACTION_SHOW_ALARMS)
                pm.queryIntentActivities(alarmIntent, 0).forEach {
                    recommendedPackages.add(it.activityInfo.packageName)
                }
                val setAlarmIntent = Intent(AlarmClock.ACTION_SET_ALARM)
                pm.queryIntentActivities(setAlarmIntent, 0).forEach {
                    recommendedPackages.add(it.activityInfo.packageName)
                }
            }
            "calendar" -> {
                val calIntent = Intent(Intent.ACTION_VIEW).setData(Uri.parse("content://com.android.calendar/time"))
                pm.queryIntentActivities(calIntent, 0).forEach {
                    recommendedPackages.add(it.activityInfo.packageName)
                }
            }
            "weather" -> {
                // Look for known weather packages or apps containing weather in name/package
                val allIntent = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
                pm.queryIntentActivities(allIntent, 0).forEach {
                    val pkg = it.activityInfo.packageName.lowercase()
                    val label = it.loadLabel(pm).toString().lowercase()
                    if (pkg.contains("weather") || pkg.contains("bom") || label.contains("weather")) {
                        recommendedPackages.add(it.activityInfo.packageName)
                    }
                }
            }
        }

        // Also query all launcher apps
        val launcherIntent = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
        val matches = pm.queryIntentActivities(launcherIntent, 0)
        val array = JSONArray()

        matches.sortedWith(compareByDescending<android.content.pm.ResolveInfo> {
            recommendedPackages.contains(it.activityInfo.packageName)
        }.thenBy {
            it.loadLabel(pm).toString().lowercase()
        }).forEach { ri ->
            val pkg = ri.activityInfo.packageName
            val label = ri.loadLabel(pm).toString()
            val isRecommended = recommendedPackages.contains(pkg)
            array.put(JSONObject().apply {
                put("package", pkg)
                put("label", label)
                put("is_recommended", isRecommended)
            })
        }

        return array.toString()
    }

    fun launchApp(context: Context, category: String, packageName: String?, eventId: Long) {
        val pm = context.packageManager

        // If specific event tap for calendar
        if (category == "calendar" && eventId > 0) {
            val eventUri = ContentUris.withAppendedId(CalendarContract.Events.CONTENT_URI, eventId)
            val intent = Intent(Intent.ACTION_VIEW).setData(eventUri).apply {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                if (!packageName.isNullOrEmpty()) {
                    setPackage(packageName)
                }
            }
            try {
                context.startActivity(intent)
                return
            } catch (e: Exception) {
                android.util.Log.w("qlaunch", "Direct event launch failed, falling back to calendar main", e)
            }
        }

        // If package name is explicitly configured, launch it
        if (!packageName.isNullOrEmpty()) {
            val launchIntent = pm.getLaunchIntentForPackage(packageName)
            if (launchIntent != null) {
                launchIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                context.startActivity(launchIntent)
                return
            }
        }

        // Fallbacks per category
        when (category.lowercase()) {
            "clock" -> {
                val intent = Intent(AlarmClock.ACTION_SHOW_ALARMS).apply {
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                }
                if (intent.resolveActivity(pm) != null) {
                    context.startActivity(intent)
                    return
                }
                // Fallback to SET_ALARM
                val setIntent = Intent(AlarmClock.ACTION_SET_ALARM).apply {
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                }
                if (setIntent.resolveActivity(pm) != null) {
                    context.startActivity(setIntent)
                    return
                }
            }
            "calendar" -> {
                val calIntent = Intent(Intent.ACTION_VIEW).setData(Uri.parse("content://com.android.calendar/time")).apply {
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                }
                if (calIntent.resolveActivity(pm) != null) {
                    context.startActivity(calIntent)
                    return
                }
            }
            "weather" -> {
                // Try Google Weather or web search fallback
                val webIntent = Intent(Intent.ACTION_VIEW, Uri.parse("https://www.google.com/search?q=weather")).apply {
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                }
                context.startActivity(webIntent)
                return
            }
        }
    }
}
