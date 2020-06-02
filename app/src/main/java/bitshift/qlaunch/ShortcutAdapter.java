package bitshift.qlaunch;

import java.util.List;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.drawable.BitmapDrawable;
import android.graphics.drawable.Drawable;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.ArrayAdapter;
import android.widget.ImageView;
import android.widget.TextView;

public class ShortcutAdapter extends ArrayAdapter<ShortcutActivity.ShortcutIntent>
{
	public ShortcutAdapter(Context context, List<ShortcutActivity.ShortcutIntent> items)
	{
		super(context, R.layout.shortcut_row, items);
	}	

    @Override
    public View getView(final int position, View convertView, ViewGroup parent) //pos final so we can access it
    {
		LayoutInflater inflater = (LayoutInflater) this.getContext().getSystemService(Context.LAYOUT_INFLATER_SERVICE); 
		convertView = inflater.inflate(R.layout.shortcut_row, null);

        ShortcutActivity.ShortcutIntent intent = (ShortcutActivity.ShortcutIntent)super.getItem(position);

	    TextView label = (TextView)convertView.findViewById(R.id.tv_label);
	    label.setText(intent.mLabel);

	    ImageView image = (ImageView)convertView.findViewById(R.id.iv_image);
	    image.setImageDrawable(resizeIcon(intent.mIcon));

	    return(convertView);
    }	
    
    Drawable resizeIcon(Bitmap bitmap)
	{
        int iconPixelSize = SettingsMgr.instance().iconSize(); // size in pixels
        Bitmap scaled = Bitmap.createScaledBitmap(bitmap, iconPixelSize, iconPixelSize, true);
		return new BitmapDrawable(scaled);
	}    
    
 
}
