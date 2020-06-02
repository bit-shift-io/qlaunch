package bitshift.qlaunch;

import java.util.UUID;

import android.content.Intent;
import android.content.pm.ActivityInfo;
import android.graphics.Bitmap;
import android.graphics.drawable.Drawable;
import android.support.v4.app.DialogFragment;
import android.support.v4.app.FragmentActivity;
import android.support.v4.app.FragmentManager;
import android.util.Log;
import android.view.View;
import android.widget.AdapterView;

public class Application extends GroupItem
{
    public Application(UUID id)
    {
        super(id);
    }

    public Application(ActivityInfo activity)
    {
        super(activity.packageName, activity.name, activity.loadLabel(PackageMgr.instance().packageManager()).toString());
    }

    @Override
    public void onItemClickInternal(AdapterView<?> adapterView, View view, int position, long id)
    {
        try
        {
            ActivityInfo info = PackageMgr.instance().packageManager().getActivityInfo(componentName(), 0); // check we exist
            if (info != null)
            {
                MainActivity.instance().closeDrawers();

                Intent intent = new Intent(Intent.ACTION_MAIN, null);
                //intent.addCategory(Intent.CATEGORY_LAUNCHER);
                Log.i(Globals.APP_NAME, "RunningApp: " + componentName().toString());
                intent.setComponent(componentName());
                intent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED);
                adapterView.getContext().startActivity(intent);
            }
        }
        catch (Exception e)
        {
            e.printStackTrace();
        }
    }

	@Override
	public boolean onItemLongClickInternal(AdapterView<?> adapterView, View view, int position, long id)
	{
        try {
            FragmentManager fragmentManager = ((FragmentActivity) adapterView.getContext()).getSupportFragmentManager();
            DialogFragment newFragment = ApplicationSettingsDialog.newInstance(this, adapterView);
            newFragment.show(fragmentManager, "dialog");
        } catch (ClassCastException e) {
            e.printStackTrace();
        }

        return true;
	}

    private Bitmap getBitmapFromDrawable(Drawable drawable) {
        return AppIconHelper.getAppIconBitmap(drawable);
    }
/*
    @Override
    Bitmap createIcon()
    {
       // if (iconOverride())
        //    return icon();

        if (icon() != null)
            return icon();

        Drawable drawable = PackageMgr.instance().applicationIcon(componentName()); // check if we exist first
        if (drawable != null)
        {
            Bitmap bitmap = getBitmapFromDrawable(drawable);
            int iconPixelSize = SettingsMgr.instance().iconSize(); // this comes in as pixels!
            Bitmap scaled = Bitmap.createScaledBitmap(bitmap, iconPixelSize, iconPixelSize, true);
            return scaled;
        }
        return null;
    }
*/

    @Override
    public void writeToFile()
    {
        JsonMgr.instance().writeApplication(this);
    }


}
