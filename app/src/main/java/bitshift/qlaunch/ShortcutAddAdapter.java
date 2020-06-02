package bitshift.qlaunch;

import java.util.List;

import android.content.Context;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.graphics.Bitmap;
import android.graphics.drawable.BitmapDrawable;
import android.graphics.drawable.Drawable;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.ArrayAdapter;
import android.widget.ImageView;
import android.widget.TextView;

public class ShortcutAddAdapter extends ArrayAdapter<ResolveInfo>
{
	public ShortcutAddAdapter(Context context, List<ResolveInfo> items)
	{
		super(context, R.layout.shortcut_row, items);
	}	

    @Override
    public View getView(final int position, View convertView, ViewGroup parent) //pos final so we can access it
    {
		LayoutInflater inflater = (LayoutInflater) this.getContext().getSystemService(Context.LAYOUT_INFLATER_SERVICE); 
		convertView = inflater.inflate(R.layout.shortcut_row, null);

		ResolveInfo app = super.getItem(position);
		PackageManager mPackageManager = this.getContext().getPackageManager();
		
	    TextView label = convertView.findViewById(R.id.tv_label);
	    label.setText(app.loadLabel(mPackageManager));

	    ImageView image = convertView.findViewById(R.id.iv_image);
	    image.setImageDrawable(resizeIcon(app.activityInfo.loadIcon(mPackageManager)));

	    return(convertView);
    }	
    
    Drawable resizeIcon(Drawable drawable)
	{
		/*
        int iconPixelSize = SettingsMgr.instance().iconSize(); // size in pixels
		Bitmap bitmap = AppIconHelper.getAppIconBitmap(drawable);
        Bitmap scaled = Bitmap.createScaledBitmap(bitmap, iconPixelSize, iconPixelSize, true);
		return new BitmapDrawable(getContext().getResources(), scaled);
		*/
		return drawable;
	}
 
}
