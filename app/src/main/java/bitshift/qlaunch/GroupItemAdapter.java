package bitshift.qlaunch;

import java.lang.ref.WeakReference;
import java.util.List;

import android.content.Context;
import android.content.res.Configuration;
import android.graphics.Bitmap;
import android.graphics.Color;
import android.os.AsyncTask;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.ArrayAdapter;
import android.widget.BaseAdapter;
import android.widget.GridView;
import android.widget.ImageView;
import android.widget.TextView;

public class GroupItemAdapter extends BaseAdapter implements Group.GroupChangeListener, MainActivity.ConfigurationChangedListener
{

    Group mGroup;
    List<GroupItem> mGroupItems;
    GridView mGrid;
    Context mContext;

	public GroupItemAdapter(Context context, Group group, GridView grid)
	{
        super();
        mContext = context;
        mGroup = group;
        mGrid = grid;
        onGroupChangeListener();
        updateGridForOrientation(context.getResources().getConfiguration().orientation);

        PackageMgr.instance().rootGroup().addGroupChangeListener(this); // root group change listener
        MainActivity.instance().addConfigurationChangedListener(this); // to listen for orientation changes
	}

    @Override
    public void onGroupChangeListener()
    {
        Log.d(Globals.APP_NAME, "onGroupChangeListener");

        // update the list of group items
        mGroupItems = mGroup.childList(GroupItem.FLAG_VISIBLE);

        notifyDataSetChanged();
    }

    @Override
    public void onConfigurationChanged(Configuration newConfig) {
        updateGridForOrientation(newConfig.orientation);
    }

    public void updateGridForOrientation(int orientation) {
        switch (orientation)
        {
            case Configuration.ORIENTATION_LANDSCAPE:
                mGrid.setNumColumns(SettingsMgr.instance().gridHeight());
                break;

            case Configuration.ORIENTATION_PORTRAIT:
                mGrid.setNumColumns(SettingsMgr.instance().gridWidth());
                break;
        }
    }

    @Override
    public int getCount() {
        return mGroupItems.size();
    }

    @Override
    public Object getItem(int position) {
        return mGroupItems.get(position);
    }

    @Override
    public long getItemId(int position) {
        return 0;
    }

    @Override
    public View getView(int position, View convertView, ViewGroup parent) 
    {
        GroupItem item = mGroupItems.get(position);
        return item.getView(mContext, convertView, parent);
    }
}


