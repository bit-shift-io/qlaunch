
/*
TODO
next version:
- clean up interfaces like in wordpad
- Slide navigation (recent apps)
- double tap to lock screen
- only backup json/xml files
-disable lockscreen

- GC_FOR_ALLOC with bitmaps - http://stackoverflow.com/questions/12716574/bitmap-recycle-with-largeheap-enabled
- Checkout this for async loading apps! http://arnab.ch/blog/2013/08/how-to-write-custom-launcher-app-in-android/

Shortcut + custom icons to SD unscaled, then we can load from there use uuid
- if bitmap override, do not scale bitmaps(or some other work around?) mainly for shortcuts & user icons OR save to SD?

- shortcuts
- Check proximity sensors not in use when screen is off (we need to do this as it keeps pinging)
- make qlaunch open the settings and put group editor etc.. in the settings
- Custom icons (image picker intenet and store the path, should be easy)
- Folders(groups)

-- Fullversion --
- shortcuts
- folders
- wallpaper settings
- font settings
- group editor
- option to launch an app on startup (ie media player etc)



CHANGELOG
Build 10
- Added support for android 7

Build 9
- Kitkat optimised (android 4.4), works with ART
- Added more debug info to package manager
- Changed page transitions
- Fixed crash when application changed icon
- Add start-up check to see if packages added or removed
- Fix bug with installing/updating app causing icon to change groups
- Added side navigation drawer to access launcher options
- Improved smoothness of scrolling
- Disabled font shadows by default (it improves performance on low end devices)
- Done more work on shortcuts, these should be working correctly now
- API minimum is now android 2.3 Gingerbread
- Added lock device shortcut and device admin manager
- General cleanup and optimisations

Build 8
- data files now stored on internal sd so backup apps should work
- removed permission to write to external sd
- changes to screen on/off code
- changed home button to not return to home screen unless launcher has focus
- fix for app view displaying incorrectly
- reduced garbage collection and update code
- rewrote code which checks added and removed apps, fixing duplicate apps

Build 7
- Default icon size to 40
- Added fade to page transition animations
- Changed the group editor dialog
- Icons are automagically scaled to screen size
- Added background opacity option
- Changed storing of apps from xml to json (massive overhaul)
- Changed the way settings, broadcast and sensor managers work
- Changed path of cache and data to /Android/data/
- Added more font and icon customisation options in preferences (font face, drop shadow, color etc)
- Added No Wallpaper option to wallpaper picker
- Newly installed apps will now appear in the app group instead of the first screen
- Hidden groups will stay hidden after updating
- When apps update they will now remember what group they were in
- Made hidden items a hidden group
- Removed unused settings
- Removed qlaunch icon from showing as an application
- Proximity sensor lock now functional
- Hidden application icons no longer stored in memory
- More work on shortcuts(currently dont save, and may be broken)


Build 6
- Fixed adding and removing apps not updating
- Home button now takes back to home screen
- Removed beta tag (however still in beta)
- Some more work done on the shortcut dialog(displays but does not work yet!)
- Apps should sort correctly now
- Prevented view resizing after exiting apps

Build 5
- Fixed check boxes on dialogs displaying incorrectly
- Added group editor
- General cleanup of code
- Added check before running app to check it exists

Build 4
- Works on android 2.0+

Build 3
- Added sorting in the hidden apps
- Menu button should now display
- Reduced API to support honeycomb (untested)
- Possible fix for boot crash on some devices

Build 2
- Fixed bug with loading duplicate apps if data was missing for the app
- Fixed bug when reinstalling rom and packagename changing
- Fixed a bug when displaying hidden apps
- Fixed bug when updating app its settings got reset
- Reduced needed permissions


 */

package bitshift.qlaunch;

import android.app.Activity;
import android.app.AlertDialog;
import android.app.job.JobInfo;
import android.app.job.JobScheduler;
import android.content.ComponentName;
import android.content.Context;
import android.content.DialogInterface;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.SharedPreferences;
import android.content.pm.ActivityInfo;
import android.content.pm.ResolveInfo;
import android.content.res.Configuration;
import android.graphics.Color;
import android.graphics.drawable.Drawable;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.preference.Preference;
import android.support.v4.app.FragmentActivity;
import android.support.v4.view.PagerTabStrip;
import android.support.v4.widget.DrawerLayout;
import android.util.DisplayMetrics;
import android.widget.GridView;
import android.widget.LinearLayout;
import android.widget.SearchView;
import android.util.Log;
import android.view.KeyEvent;
import android.view.LayoutInflater;
import android.view.Menu;
import android.view.View;
import android.view.Window;
import android.view.WindowManager;
import android.widget.AdapterView;
import android.widget.ImageView;
import android.widget.ListView;
import android.widget.TextView;
import android.content.pm.ChangedPackages;

import java.lang.reflect.Method;

import java.io.File;
import java.util.ArrayList;
import java.util.List;


public class MainActivity extends FragmentActivity implements Group.GroupChangeListener, SettingsMgr.SettingsMgrListener, IntentMgr.ShortcutListener
{
    // custom listener interface (interface is a group of related methods with empty bodies)
    public interface ConfigurationChangedListener
    {
        void onConfigurationChanged(Configuration newConfig);
    }


    private static final int JOB_ID = 1337;

    static private MainActivity mSingleInstance;

    // views
    private DrawerLayout mDrawerLayout;
    private ListView mLeftDrawer;
    private LinearLayout mRightDrawer;

    // right drawer
    private AllAppsAdapter mAllAppsAdapter;
    private SearchView mAllAppsSearchView;
    private GridView mAllAppsView;

    SectionsPagerAdapter mSectionsPagerAdapter;
    CycleViewPager mViewPager;

    private static PagerTabStrip mPagerTabStrip;
    private static ImageView mBackground;

    public static String BASEDIR; // store the INTERNAL files dir here
    private boolean mInFocus = true;

    // managers
    public static IntentMgr mIntentMgr;
    private static SensorMgr mSensorMgr;
    private static PackageMgr mPackageMgr;
    private static SettingsMgr mSettingsMgr;
    private static GroupItemMgr mGroupItemMgr;
    private static JsonMgr mJsonMgr;
    private static NavigationMgr mNavigationMgr;
    private static DeviceAdminMgr mDeviceAdminMgr;

    private Handler mPollHandler;
    private Runnable mPollRunnable;

    final public static int REQUEST_CODE_SHORTCUT = 0;
    final public static int REQUEST_CODE_IMAGE = 1;
    final public static int REQUEST_IMPORT_SETTINGS_DIR = 2;

    private int mLastPackageSequenceNumber = 0;

    enum StatusBarState {
        Open,
        Closed,
    }
    StatusBarState mStatusBarState = StatusBarState.Closed;


    private ArrayList<ConfigurationChangedListener> mConfigurationChangedListeners = new ArrayList<ConfigurationChangedListener> ();

    public void addConfigurationChangedListener(ConfigurationChangedListener listener)
    {
        for(int i = 0; i < mConfigurationChangedListeners.size(); ++i)
            if (mConfigurationChangedListeners.get(i).equals(listener));
        mConfigurationChangedListeners.remove(listener);

        mConfigurationChangedListeners.add(listener);
    }

    public void onWindowFocusChanged(boolean hasFocus) {
        mInFocus = hasFocus;
    }

    // A method to find height of the status bar
    public int getStatusBarHeight() {
        int resourceId = getResources().getIdentifier("status_bar_height", "dimen", "android");
        if (resourceId > 0) {
            return getResources().getDimensionPixelSize(resourceId);
        }
        return 0;
    }

    private int getSoftButtonsBarHeight() {
        int resourceId = getResources().getIdentifier("navigation_bar_height", "dimen", "android");
        if (resourceId > 0) {
            return getResources().getDimensionPixelSize(resourceId);
        }
        return 0;
    }

    static MainActivity instance()
    {
        return mSingleInstance;
    }

    // onCreate called by android, thread the loading of create() via delay load handler
	@Override
	protected void onCreate(Bundle savedInstanceState) 
	{
        mSingleInstance = this;

 		super.onCreate(savedInstanceState);
        Log.i(Globals.APP_NAME, "onCreate()");

        requestWindowFeature(Window.FEATURE_NO_TITLE);

        // make status bar totally transparent
        Window w = getWindow(); // in Activity's onCreate() for instance
        w.setFlags(WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS, WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS);

        // load in views
        setContentView(R.layout.activity_main);
        mPagerTabStrip = findViewById(R.id.titlepageindicator);

        // setup padding on the centre view
        LinearLayout centreLayout = findViewById(R.id.centre_layout);
        centreLayout.setPadding(0, getStatusBarHeight(), 0, getSoftButtonsBarHeight());


        mViewPager = findViewById(R.id.viewpager);

        mBackground = findViewById(R.id.background);
        mPagerTabStrip.setVisibility(View.GONE);
        mBackground.setVisibility(View.GONE);

        // navigation views (left drawer)
        mDrawerLayout = findViewById(R.id.drawer_layout);
        mLeftDrawer = findViewById(R.id.left_drawer);
        mLeftDrawer.setPadding(0, getStatusBarHeight(), 0, getSoftButtonsBarHeight());


        BASEDIR = getApplicationContext().getFilesDir().toString() + "/";

        // create folders
        try
        {
            File f = new File(BASEDIR + "data/application/");
            f.mkdirs();

            f = new File(BASEDIR + "data/group/");
            f.mkdirs();

            f = new File(BASEDIR + "data/shortcut/");
            f.mkdirs();

            f = new File(BASEDIR + "res/");
            f.mkdirs();
        }
        catch (Exception e)
        {
            e.printStackTrace();
        }

        // setup our helper singletons
        // NOTE: storing context can be bad, make sure we try to eliminate the need for them in these!!
        mDeviceAdminMgr = new DeviceAdminMgr(getApplicationContext());

        mIntentMgr = new IntentMgr();


        mSensorMgr = new SensorMgr(getApplicationContext());
        mSettingsMgr = new SettingsMgr(getApplicationContext());
        mGroupItemMgr = new GroupItemMgr();
        mPackageMgr = new PackageMgr(getPackageManager(), getApplicationContext());
        mJsonMgr = new JsonMgr(getApplicationContext());
        mNavigationMgr = new NavigationMgr();

        // listeners
        mIntentMgr.addPackageListener(mPackageMgr); // package manager to be notified
        mIntentMgr.addShortcutListener(this);
        mSettingsMgr.addSettingsChangeListener(this);
        mSettingsMgr.addSettingsChangeListener(mPackageMgr);
        mPackageMgr.rootGroup().addGroupChangeListener(this); // root group change listener

        // update app list after complete reboot
        mPackageMgr.updatePackageList();


        // setup the centre view pager adapter
        List<GroupItem> list = PackageMgr.instance().rootGroup().childList(GroupItem.FLAG_VISIBLE);
        mSectionsPagerAdapter = new SectionsPagerAdapter(list, mViewPager);
        mViewPager.setAdapter(mSectionsPagerAdapter);

        // TODO: this seems to cause issues as sometimes the icons dont appear
        mViewPager.setPageTransformer(true, mSectionsPagerAdapter);


        updateTabStripVisibilityForOrientation(getResources().getConfiguration().orientation);

        createAllAppsDrawer();
        createNavigation();
        drawBackground();

        updateSettings();
        //goHomeScreen();

        // polling for package changes
        // Create the Handler object (on the main thread by default)
        mPollHandler = new Handler();
        // Define the code block to be executed
        mPollRunnable = new Runnable() {
            @Override
            public void run() {
                // Do something here on the main thread
                //Log.d("Handlers", "Called on main thread");
                poll();
                // Repeat this the same runnable code block again another 2 seconds
                // 'this' is referencing the Runnable object
                mPollHandler.postDelayed(this, 3000);
            }
        };
        // Start the initial runnable task by posting through the handler
        setPollingEnabled(true);


        /*
        new Thread() {
            @Override
            public void run() {
                try {
                    while (true) {
                        Thread.sleep(1000);
                        //System.out.println(LocalTime.getTime().format2445());
                        runOnUiThread(new Runnable() {
                            @Override
                            public void run() {
                                poll();
                            }
                        });
                    }
                } catch (InterruptedException e) {
                    e.printStackTrace();
                }
            }
        }.start();*/
	}

	private void createAllAppsDrawer()
    {
        mRightDrawer = findViewById(R.id.right_drawer);
        mRightDrawer.setPadding(0, getStatusBarHeight(), 0, getSoftButtonsBarHeight());

        // all apps drawer (right drawer)
        mAllAppsSearchView = findViewById(R.id.all_apps_search_view);
        mAllAppsView = findViewById(R.id.all_apps_view);

        // setup the all apps drawer adapter
        List<GroupItem> all_applications = PackageMgr.instance().getAllApplications();
        mAllAppsAdapter = new AllAppsAdapter(getApplicationContext(), all_applications);
        mAllAppsSearchView.setOnQueryTextListener(mAllAppsAdapter);
        mAllAppsView.setNumColumns(3);
        mAllAppsView.setAdapter(mAllAppsAdapter);
        mAllAppsView.setOnItemClickListener(mAllAppsAdapter);
        mAllAppsView.setOnItemLongClickListener(mAllAppsAdapter);
    }

    private void createNavigation()
    {
        // setup the navigation drawer
        NavigationItemAdapter adapter = new NavigationItemAdapter(getApplicationContext());
        mLeftDrawer.setAdapter(adapter);
        mLeftDrawer.setOnItemClickListener(new AdapterView.OnItemClickListener() {
            public void onItemClick(AdapterView<?> adapterView, View view, int position, long id) {
                NavigationItem item = (NavigationItem) adapterView.getItemAtPosition(position);
                SelectNavigationItem(item);
                mDrawerLayout.closeDrawer(mLeftDrawer);
            }
        });
    }


    void SelectNavigationItem(NavigationItem item)
    {
        switch (item.Id())
        {
            case R.id.menu_add_shortcut:
                addShortcut();
                break;

            case R.id.menu_group_editor:
                groupEditor();
                break;

            case R.id.menu_select_wallpaper:
                selectWallpaper();
                break;

            case R.id.menu_launcher_settings:
                launcherSettings();
                break;
        }
    }

    void selectWallpaper()
    {
        Intent wallpaperIntent = new Intent(Intent.ACTION_SET_WALLPAPER);
        String select = getResources().getString(R.string.dialog_title_select_wallpaper);
        startActivity(Intent.createChooser(wallpaperIntent, select));
    }

    void groupEditor()
    {
        Intent groupIntent = new Intent(this, GroupEditorActivity.class);
        startActivity(groupIntent);
    }

    void launcherSettings()
    {
        Intent settingsIntent = new Intent(this, SettingsActivity.class);
        startActivity(settingsIntent);
    }

	void addShortcut()
	{
		// inflate the builder
		LayoutInflater inflater = LayoutInflater.from(this);
        final View dialogView = inflater.inflate(R.layout.dialog_shortcut_add, null);

        AlertDialog.Builder builder = new AlertDialog.Builder(this);
        builder.setView(dialogView);
        builder.setNegativeButton(R.string.cancel, null);

        final AlertDialog dialog = builder.create(); // store dialog so we can close it after!
        final List<ResolveInfo> items = getPackageManager().queryIntentActivities(new Intent(Intent.ACTION_CREATE_SHORTCUT), 0);
		ListView listShortcut = (ListView) dialogView.findViewById(R.id.list_shortcut);
		ShortcutAddAdapter adapter = new ShortcutAddAdapter(this, items);
		listShortcut.setAdapter(adapter);
        listShortcut.setOnItemClickListener(new AdapterView.OnItemClickListener()
		{
			@Override
			public void onItemClick(AdapterView<?> adapterView, View view, int position, long id) 
			{
                dialog.dismiss();
                //dialog.cancel();

				//unregisterReceiver(mBroadcastMgr);
				ComponentName componentName = new ComponentName(items.get(position).activityInfo.packageName, items.get(position).activityInfo.name);
				try 
				{
					Intent intent = new Intent(Intent.ACTION_CREATE_SHORTCUT);
                    //intent.addCategory(Intent.CATEGORY_DEFAULT);
                    //intent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED);
					intent.setComponent(componentName);
					((Activity) adapterView.getContext()).startActivityForResult(intent, REQUEST_CODE_SHORTCUT);
				} 
				catch (Exception e)
                {
                    e.printStackTrace();
                }
			}
        });

        dialog.show();
	}


    @Override
    public void onShortcutListener(Intent intent)
    {
        onActivityResult(REQUEST_CODE_SHORTCUT, Activity.RESULT_OK, intent);
    }

	@Override 
	public void onActivityResult(int requestCode, int resultCode, Intent intent)
	{     
		super.onActivityResult(requestCode, resultCode, intent);
	
		if (resultCode != Activity.RESULT_OK) // check we have picked something
			return;
		
		switch(requestCode) 
		{
            case REQUEST_IMPORT_SETTINGS_DIR:
            {
                Log.i(Globals.APP_NAME, "Import settings from: " + intent.getData());
                break;
            }

			case REQUEST_CODE_SHORTCUT:
			{
                int GroupId = mViewPager.getCurrentItem();
                Group parent = (Group) PackageMgr.instance().rootGroup().childList(GroupItem.FLAG_VISIBLE).get(GroupId);

                Shortcut shortcut = new Shortcut(intent);
                if (!shortcut.isCategoryLauncher())
                {
                    parent.add(shortcut);
                    parent.update();
                    shortcut.writeToFile();
                }
                else
                    Log.i(Globals.APP_NAME, "Ignore: Android market shortcut");
				break; 
			}
		} 
	
	}

	void poll()
    {
        Log.i(Globals.APP_NAME, "poll");

        if (Build.VERSION.SDK_INT < 26){
            mPackageMgr.updatePackageList();
        }
        else {
            // check for package changes
            ChangedPackages delta = getPackageManager().getChangedPackages(mLastPackageSequenceNumber);
            mLastPackageSequenceNumber = (delta == null) ? 0 : delta.getSequenceNumber();
            if (delta != null) {
                mPackageMgr.updatePackageList();
            }
        }
    }

	void update()
	{
        Log.i(Globals.APP_NAME, "update() in MainActivity");

        if (mSectionsPagerAdapter != null) {
            mSectionsPagerAdapter.setItemList(PackageMgr.instance().rootGroup().childList(GroupItem.FLAG_VISIBLE));
        }
	}

    void hideTabStrip()
    {
        mPagerTabStrip.setVisibility(PagerTabStrip.GONE);
    }

    void drawTabStrip()
    {
        int color = Color.TRANSPARENT; //Color.parseColor("#99000000"); // 25%
        mPagerTabStrip.setBackgroundColor(color);
        mPagerTabStrip.setDrawFullUnderline(false);
        //mPagerTabStrip.setDraw
        mPagerTabStrip.setTabIndicatorColor(Color.parseColor("#99000000"));

        for (int i = 0; i < mPagerTabStrip.getChildCount(); ++i)  // loop over children and give a font
        {
            View nextChild = mPagerTabStrip.getChildAt(i);
            if (nextChild instanceof TextView)
            {
                TextView label = (TextView) nextChild;
                label.setTypeface(SettingsMgr.instance().fontTypeface(), 0);
                if (SettingsMgr.instance().fontShadowEnabled())
                    label.setShadowLayer(1, 3, 3, Color.BLACK);
            }
        }

        mPagerTabStrip.setVisibility(PagerTabStrip.VISIBLE);
    }

    void drawBackground()
    {
        if (SettingsMgr.instance().wallpaperAlpha() == 0)
            mBackground.setVisibility(View.GONE);
        else
        {
            mBackground.setVisibility(View.VISIBLE);
            Drawable background = mBackground.getDrawable();
            background.setAlpha(SettingsMgr.instance().wallpaperAlpha());
        }
    }

	// Activity config change in manifest: android:configChanges="orientation|screenSize"
	@Override
    public void onConfigurationChanged(Configuration newConfig)
	{
		super.onConfigurationChanged(newConfig);

        // update the tab strip visibility
        int orientation = newConfig.orientation;
        updateTabStripVisibilityForOrientation(orientation);

        // trigger listeners - really the GroupItemAdapters to cause the grid layout changes
        for (ConfigurationChangedListener listener : mConfigurationChangedListeners) {
            listener.onConfigurationChanged(newConfig);
        }
    }

    private void updateTabStripVisibilityForOrientation(int orientation) {
        String strTabDisplay = SettingsMgr.instance().displayTabStrip(); // 0 - always, -1 never, 1 landscape only, 2 portrait only
        switch (orientation)
        {
            case Configuration.ORIENTATION_LANDSCAPE:
            {
                if (strTabDisplay.equals("0") || strTabDisplay.equals("1"))
                    drawTabStrip();
                else
                    hideTabStrip();
                break;
            }
            case Configuration.ORIENTATION_PORTRAIT:
            {
                if (strTabDisplay.equals("0") || strTabDisplay.equals("2"))
                    drawTabStrip();
                else
                    hideTabStrip();
                break;
            }
        }
    }

	@Override
	public boolean onCreateOptionsMenu(Menu menu) 
	{
        //getMenuInflater().inflate(R.menu.menu_main, menu); // Inflate the menu; this adds items to the action bar if it is present.
        return true;
	}

	@Override
	public void onGroupChangeListener() 
	{
		update();
	}

    // triggered on screen on
    @Override
    public void onResume()
    {
        super.onResume();
        setPollingEnabled(true);
/*
        IntentFilter packageIntentFilters = new IntentFilter();
        packageIntentFilters.addAction(Intent.ACTION_PACKAGE_REMOVED);
        packageIntentFilters.addAction(Intent.ACTION_PACKAGE_REPLACED);
        packageIntentFilters.addAction(Intent.ACTION_CREATE_SHORTCUT);
        registerReceiver(mIntentMgr, packageIntentFilters);

        /*
        // check for package changes
        ChangedPackages delta = getPackageManager().getChangedPackages(mLastPackageSequenceNumber);
        mLastPackageSequenceNumber = (delta == null) ? 0 : delta.getSequenceNumber();
        if (delta != null) {
            mPackageMgr.updatePackageList();
        }* /
        mPackageMgr.updatePackageList();
*/
        update();
    }

    void setPollingEnabled(boolean bEnabled) {
        if (bEnabled) {
            mPollHandler.post(mPollRunnable);
        }
        else {
            mPollHandler.removeCallbacks(mPollRunnable);
        }
    }

    // triggered on screen off
    @Override
    public void onPause()
    {
        super.onPause();
        setPollingEnabled(false);
        System.gc();
    }

    // triggered on refocus from app
    @Override
    public void onRestart()
    {
        super.onRestart();
        setPollingEnabled(true);
        //mInFocus = true;
    }

    // triggered when other app takes focus
    @Override
    public void onStop()
    {
        super.onStop();
        setPollingEnabled(false);
        //mInFocus = false;
    }

    public void clearApplicationData() {
        File cacheDirectory = getCacheDir();
        File applicationDirectory = new File(cacheDirectory.getParent());
        if (applicationDirectory.exists()) {
            String[] fileNames = applicationDirectory.list();
            for (String fileName : fileNames) {
                if (!fileName.equals("lib")) {
                    deleteFile(new File(applicationDirectory, fileName));
                }
            }
        }
    }

    public static boolean deleteFile(File file) {
        boolean deletedAll = true;
        if (file != null) {
            if (file.isDirectory()) {
                String[] children = file.list();
                for (int i = 0; i < children.length; i++) {
                    deletedAll = deleteFile(new File(file, children[i])) && deletedAll;
                }
            } else {
                deletedAll = file.delete();
            }
        }

        return deletedAll;
    }

    public void onPreferenceClick(SettingsActivity.SettingsFragment settingsFragment, Preference preference)
    {
        String key = preference.getKey();
        if (key.equals("key_factory_reset_button")) {
            settingsFragment.close();

            new AlertDialog.Builder(this)
                    .setTitle("Clear Application Data?")
                    .setMessage("Are you sure you want to wipe all data for QLaunch?")
                    .setIcon(android.R.drawable.ic_dialog_alert)
                    .setPositiveButton(android.R.string.yes, new DialogInterface.OnClickListener() {

                        public void onClick(DialogInterface dialog, int whichButton) {
                            clearApplicationData();
                            mSettingsMgr.preferences().edit().clear().commit();
                            Log.i(Globals.APP_NAME, "Factory reset pressed, data has been deleted, killing launcher");
                            android.os.Process.killProcess(android.os.Process.myPid());
                            System.exit(0);
                        }})
                    .setNegativeButton(android.R.string.no, null).show();
        }
        else if (key.equals("key_restart_launcher"))
        {
            Log.i(Globals.APP_NAME, "Restart pressed, exiting launcher");
            System.exit(0);
        }
    }

    @Override
    public void onSettingsChanged(SharedPreferences sharedPreferences, String key) {
        if (key.equals("key_wallpaper_alpha"))
        {
            drawBackground();
        }

        updateSettings();
    }

    void updateSettings()
    {
        // full screen - notification bar
        if (SettingsMgr.instance().displayNotificationBar())
        {
            getWindow().addFlags(WindowManager.LayoutParams.FLAG_FORCE_NOT_FULLSCREEN);
            getWindow().clearFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN);
        }
        else
        {
            getWindow().addFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN);
            getWindow().clearFlags(WindowManager.LayoutParams.FLAG_FORCE_NOT_FULLSCREEN);
        }

        // force orientation
        String strOrientation = SettingsMgr.instance().screenOrientation();
        if (strOrientation.equals("0"))
            setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_SENSOR);
        else if (strOrientation.equals("1"))
            setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE);
        else if (strOrientation.equals("2"))
            setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_PORTRAIT);

        update();
    }

    // Key Presses
    @Override
    public boolean onKeyDown(int keyCode, KeyEvent event)
    {
        if (!mSensorMgr.checkProximity())
            return true;

        switch(keyCode) // change from false to not allow android to continue doing what it does
        {
            case KeyEvent.KEYCODE_BACK: {
                //goHomeScreen();
                return true;
            }
        }

        return super.onKeyDown(keyCode, event);
    }


    // detect home press in here, and point to our broadcast manager
    @Override
    protected void onNewIntent(Intent intent)
    {
        super.onNewIntent(intent);
        if (mInFocus) {
            goHomeScreen();
        }
    }

    public void toggleStatusBar()
    {
        if (!SettingsMgr.instance().toggleStatusBar()) {
            return;
        }

        if (mStatusBarState == StatusBarState.Closed) {
            expandStatusBar();
            mStatusBarState = StatusBarState.Open;
        }
        else if (mStatusBarState == StatusBarState.Open) {
            collapseStatusBar();
            mStatusBarState = StatusBarState.Closed;
        }
    }

    public void expandStatusBar()
    {
        try
        {
            Object sbservice = getSystemService("statusbar");
            Class<?> statusbarManager = Class.forName("android.app.StatusBarManager");
            Method method;
            if (Build.VERSION.SDK_INT >= 17) {
                method = statusbarManager.getMethod("expandNotificationsPanel");
            } else {
                method = statusbarManager.getMethod("expand");
            }
            method.invoke(sbservice);
        }
        catch (Exception e)
        {
            e.printStackTrace();
        }
    }

    public void collapseStatusBar()
    {
        Intent it = new Intent(Intent.ACTION_CLOSE_SYSTEM_DIALOGS);
        sendBroadcast(it);
    }

    boolean closeDrawers() {
        boolean wasOpen = false;

        // close drawers that might be open
        if (mDrawerLayout.isDrawerOpen(mLeftDrawer)) {
            mDrawerLayout.closeDrawer(mLeftDrawer);
            wasOpen = true;
        }

        if (mDrawerLayout.isDrawerOpen(mRightDrawer)) {
            mDrawerLayout.closeDrawer(mRightDrawer);
            wasOpen = true;
        }

        return wasOpen;
    }

    void goHomeScreen()
    {
        int homeScreenPos = homeScreen() + 1; // as the Circular adapter ads a fake page
        int realPosition = mViewPager.getCurrentItem();

        // if a draw was open, pressing home closes it and thats all
        if (closeDrawers())
            return;

        // pressing home on the home screen gives us a button to do something fancy with!
        if (realPosition == homeScreenPos)
        {
            toggleStatusBar();
            return;
        }

        // finally, it will take us home
        mViewPager.setCurrentItem(homeScreenPos);
    }

    int homeScreen()
    {
        List<Integer> homeGroup = PackageMgr.instance().rootGroup().childIndexList(GroupItem.FLAG_HOMESCREENGROUP);
        return homeGroup.get(0);
    }


}
