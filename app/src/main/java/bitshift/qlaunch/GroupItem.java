package bitshift.qlaunch;

import android.content.ComponentName;
import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.Color;
import android.graphics.drawable.BitmapDrawable;
import android.graphics.drawable.Drawable;
import android.os.AsyncTask;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.AdapterView;
import android.widget.GridView;
import android.widget.ImageView;
import android.widget.TextView;

import java.io.File;
import java.io.FileOutputStream;
import java.lang.ref.WeakReference;
import java.util.UUID;

public class GroupItem //an empty shell, like fab - he says no code is good code!
{
    public static interface Visitor
    {
        public void visit(GroupItem item);
    }

    public enum Type
    {
        Group,
        Application,
        Shortcut,
        Widget
    }

    public static final int FLAG_NONE = 0;
    public static final int FLAG_HIDDEN = 0x1 << 0;
    public static final int FLAG_VISIBLE = 0x1 << 1;
    public static final int FLAG_ALL = GroupItem.FLAG_VISIBLE | GroupItem.FLAG_HIDDEN;
    public static final int FLAG_UNINSTALLED = 0x1 << 2;
    public static final int FLAG_APPGROUP = 0x1 << 3;
    public static final int FLAG_GAMEGROUP = 0x1 << 4;
    public static final int FLAG_HOMESCREENGROUP = 0x1 << 5;
    public static final int FLAG_ROOTGROUP = 0x1 << 6;

    private UUID mId;
	private String mLabel = "";
	private Group mParent;
	//private Bitmap mIcon;
    private boolean mIconOverride = false; // use this if we have an icon in the database!
	private long mOverridePosition = -1;
    private long mFlags = FLAG_VISIBLE;
	private String mClassName;
	private String mPackageName;
    private String mCategory;


    public GroupItem()
    {
        setId();
    }

    public GroupItem(UUID id)
    {
        setId(id);
    }

	public GroupItem(String packageName, String className)
	{
        setId();
        setPackageName(packageName);
        setClassName(className);
	}

    public GroupItem(String packageName, String className, String label)
    {
        setId();
        setPackageName(packageName);
        setClassName(className);
        setLabel(label);
    }

    public boolean testFlag(int flag)
    {
        return (mFlags & flag) != 0;
    }

    public long flags()
    {
        return mFlags;
    }

    public void setFlags(long flags)
    {
        mFlags = flags;
    }

    public void setFlag(int flag, boolean enable)
    {
        // visible hack - as these are mutually exclusive
        if ((flag & FLAG_HIDDEN) != 0)
        {
            mFlags &= ~FLAG_VISIBLE;
        }
        else if ((flag & FLAG_VISIBLE) != 0)
        {
            mFlags &= ~FLAG_HIDDEN;
        }

        if (enable)
            mFlags |= flag;
        else
            mFlags &= ~flag;
    }

    protected void setId(UUID id)
    {
        mId = id;
    }

    protected void setPackageName(String packageName)
    {
        if (packageName == null || mPackageName != null)
            return;

        mPackageName = packageName;
    }

    protected void setClassName(String className)
    {
        mClassName = className;
    }

    void setLabel()
    {
        if (label() == null || label().equals(""))
        {
            String label = PackageMgr.instance().applicationLabel(componentName());
            setLabel(label);
        }
    }

    ComponentName componentName()
    {
        ComponentName component = null;

        if (packageName() != null && className() != null)
            component = new ComponentName(packageName(), className());

        return component;
    }

    void setCategory(String string) { mCategory = string; }

    String category() { return mCategory; }

	void setLabel(String string)
	{
		mLabel = string;
	}

    public boolean iconOverride()
    {
        return mIconOverride;
    }

    public void setIconOverride(boolean bool)
    {
        mIconOverride = bool;
    }

    // reads icon from file
    public Bitmap overrideIcon()
    {
        String fName = MainActivity.BASEDIR + "res/" + id().toString() + ".png";
        return BitmapFactory.decodeFile(fName);
    }


    // saves a bitmap
    public void outputIconOverride(Bitmap bitmap)
    {
        String fName = MainActivity.BASEDIR + "res/" + id().toString() + ".png";

        try
        {
            FileOutputStream output = new FileOutputStream(fName);
            bitmap.compress(Bitmap.CompressFormat.PNG, 100, output);
        }
        catch (Exception e)
        {
            e.printStackTrace();
        }
    }

    // as above, but converts drawable to bitmap first
    public void outputIconOverride(Drawable drawable)
    {
        if (drawable == null)
            return;

        Bitmap bitmap = AppIconHelper.getAppIconBitmap(drawable);
        outputIconOverride(bitmap);
    }
/*
    Bitmap icon()
    {
        return mIcon;
    }

    Bitmap createIcon()
    {
        return icon();
    }

    void deleteIcon()
    {
        if (mIcon == null)
            return;

        mIcon.recycle();
        mIcon = null;

        String iconPath = MainActivity.BASEDIR + "data/res/" + id().toString();
        File f = new File(iconPath);
        if (f.exists())
            f.delete();
    }

    void setIcon(Bitmap bitmap)
    {
        if (bitmap == null)
            return;

        mIcon = bitmap;
    }

	void setIcon(Drawable drawable)
	{
        if (drawable == null)
            return;

    	Bitmap bitmap = ((BitmapDrawable) drawable).getBitmap();
        setIcon(bitmap);
	}*/

    void setParent(Group group)
    {
        mParent = group;
    }

	String label()
	{
		return mLabel;
	}
	
	String className()
	{
		return mClassName;
	}

	Group parent()
	{
		return mParent;
	}
	
	String packageName()
	{
		return mPackageName;
	}

    public void onItemClick(AdapterView<?> arg0, View view, int position, long id)
    {
        // proximity check
        if (!SensorMgr.instance().checkProximity())
            return;

        onItemClickInternal(arg0, view, position, id);
    }

    public void onItemClickInternal(AdapterView<?> adapterView, View view, int position, long id)
    {
    }

    public boolean onItemLongClick(AdapterView<?> arg0, View view, int position, long id)
    {
        // proximity check
        if (!SensorMgr.instance().checkProximity())
            return true;

        // locked check
        if (SensorMgr.instance().checkLock())
            return true;

        return onItemLongClickInternal(arg0, view, position, id);
    }

    public boolean onItemLongClickInternal(AdapterView<?> arg0, View view, int position, long id)
    {
        return true;
    }

    // a view holder which holds the view information
    // this is an optimisation to speed the scrolling in the list view
    // http://developer.android.com/training/improving-layouts/smooth-scrolling.html
    static class ViewHolder
    {
        //GroupItem groupItem;
        TextView label;
        ImageView icon;
    }

/*
    // bitmap downloader task
    // this loads the bitmaps on a separate thread
    // http://developer.android.com/training/improving-layouts/smooth-scrolling.html
    static class BitmapDownloaderTask extends AsyncTask<String, Void, Bitmap>
    {
        // TODO: remove weak references, see note here http://developer.android.com/training/displaying-bitmaps/cache-bitmap.html
        private final WeakReference<ImageView> imageViewReference;
        private final WeakReference<GroupItem> groupItemReference;

        public BitmapDownloaderTask(ImageView imageView, GroupItem item)
        {
            imageViewReference = new WeakReference<ImageView>(imageView);
            groupItemReference = new WeakReference<GroupItem>(item);
        }

        @Override
        // Actual download method, run in the task thread
        protected Bitmap doInBackground(String... params)
        {
            int i = 0;
            if (groupItemReference != null)
            {
                GroupItem item = groupItemReference.get();
                if (item != null)
                {
                    Bitmap bitmap = item.createIcon();
                    return bitmap;
                }
            }
            return null;
        }

        @Override
        // Once the image is downloaded, associates it to the imageView
        protected void onPostExecute(Bitmap bitmap)
        {
            if (isCancelled())
            {
                bitmap = null;
            }

            if (imageViewReference != null)
            {
                ImageView imageView = imageViewReference.get();
                if (imageView != null)
                {
                    if (bitmap != null)
                    {
                        //imageView.setAlpha(SettingsMgr.instance().iconAlpha());
                        imageView.setImageBitmap(bitmap);
                        groupItemReference.get().setIcon(bitmap);
                    }
                }
            }
        }
    }


    // bitmap cache
    // uses more memory, but will smooth the scrolling
    // http://developer.android.com/training/displaying-bitmaps/cache-bitmap.html
*/

    public View getView(Context context, View convertView, ViewGroup parent)
    {
        ViewHolder viewHolder; // class below
        if (convertView == null)
        {
            LayoutInflater inflater = (LayoutInflater) context.getSystemService(Context.LAYOUT_INFLATER_SERVICE);
            convertView = inflater.inflate(SettingsMgr.instance().tileLayout(), parent, false); // mLayout = R.layout.app_tile

            // setups the view holder
            viewHolder = new ViewHolder();

            viewHolder.label = convertView.findViewById(R.id.appLabel);
            viewHolder.icon = convertView.findViewById(R.id.appIcon);

            // store stuff in the view, this stuff only needs to be done once
            // FONT Layout
            if (SettingsMgr.instance().fontEnabled())
            {
                viewHolder.label.setTextSize(SettingsMgr.instance().fontSize());
                viewHolder.label.setTypeface(SettingsMgr.instance().fontTypeface(), 0);
                viewHolder.label.setTextColor(SettingsMgr.instance().fontColor());

                if (SettingsMgr.instance().fontShadowEnabled())
                    viewHolder.label.setShadowLayer(1, 3, 3, Color.BLACK);
                else
                    viewHolder.label.setShadowLayer(0, 0, 0, Color.WHITE);
            }
            else
                viewHolder.label.setVisibility(TextView.GONE);

            // ICON Layout
            if (SettingsMgr.instance().iconEnabled())
            {
                ViewGroup.LayoutParams params = viewHolder.icon.getLayoutParams();
                params.width = SettingsMgr.instance().iconSize();
                params.height = SettingsMgr.instance().iconSize();
                viewHolder.icon.setLayoutParams(params);
                viewHolder.icon.setAlpha(SettingsMgr.instance().iconAlpha());
            }
            else
                viewHolder.icon.setVisibility(ImageView.GONE);

            // store the viewholder with the view
            convertView.setTag(viewHolder);
        }
        else
        {
            viewHolder = (ViewHolder) convertView.getTag();
        }

        viewHolder.label.setText(label());
/*
        if (icon() == null)
        {
            BitmapDownloaderTask task = new BitmapDownloaderTask(viewHolder.icon, this);
            task.execute();
        }
        viewHolder.icon.setImageBitmap(icon());*/

        viewHolder.icon.setImageDrawable(applicationIcon());
        return convertView;
    }
	
	public void update()
	{
		if (mParent != null)
			mParent.update();
	}

    public UUID id()
    {
        return mId;
    }

    public void setId()
    {
        UUID uid = UUID.randomUUID();
        setId(uid);
    }

	void setOverridePosition(long id)
	{
		mOverridePosition = id;
	}
	
	Long overridePosition()
	{
		return mOverridePosition;
	}

	public Group getGroup(int type)
    {
        Group root = PackageMgr.instance().rootGroup();
        return (Group) root.childList(type).get(0);
    }

	public Group appGroup()
	{
        Group root = PackageMgr.instance().rootGroup();
        return (Group) root.childList(GroupItem.FLAG_APPGROUP).get(0);
	}

    public Group gameGroup()
    {
        Group root = PackageMgr.instance().rootGroup();
        return (Group) root.childList(GroupItem.FLAG_GAMEGROUP).get(0);
    }

    public Group homeGroup()
    {
        Group root = PackageMgr.instance().rootGroup();
        return (Group) root.childList(GroupItem.FLAG_HOMESCREENGROUP).get(0);
    }

    public void acceptVisitor(Visitor visitor)
    {
        visitor.visit(this);
    }

    public void writeToFile()
    {
    }

    public void deleteFile()
    {
        JsonMgr.instance().deleteItemFile(this);
    }

    public void delete()
    {
        setFlag(GroupItem.FLAG_UNINSTALLED, true);
        writeToFile();
        Group parent = parent();
        parent().remove(this);
        parent.update();
    }


    /*
    If we need to scale this look at: https://developer.android.com/reference/android/graphics/drawable/ScaleDrawable.html
     */
    Drawable applicationIcon()
    {
        Drawable icon = null;
        try
        {
            icon = PackageMgr.instance().packageManager().getApplicationIcon(mPackageName);
        }
        catch (Exception e)
        {
            e.printStackTrace();
        }
        return icon;
    }

}
