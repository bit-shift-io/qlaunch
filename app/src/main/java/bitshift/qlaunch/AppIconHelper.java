package bitshift.qlaunch;

import android.content.pm.PackageManager;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.drawable.AdaptiveIconDrawable;
import android.graphics.drawable.BitmapDrawable;
import android.graphics.drawable.Drawable;
import android.graphics.drawable.LayerDrawable;
import android.os.Build;
import android.support.annotation.RequiresApi;

/**
 * Created by fabian on 18/02/18.
 * https://stackoverflow.com/questions/44447056/convert-adaptiveicondrawable-to-bitmap-in-android-o-preview
 */

public class AppIconHelper {

    public static Bitmap getAppIconBitmap(Drawable drawable) {

        if (Build.VERSION.SDK_INT >= 26) {
            return getAppIconBitmapV26(drawable);
        }

        return ((BitmapDrawable) drawable).getBitmap();
    }

    @RequiresApi(api = Build.VERSION_CODES.O)
    public static Bitmap getAppIconBitmapV26(Drawable drawable) {

        if (drawable instanceof BitmapDrawable) {
            return ((BitmapDrawable) drawable).getBitmap();
        } else if (drawable instanceof AdaptiveIconDrawable) {

            final Bitmap bmp = Bitmap.createBitmap(drawable.getIntrinsicWidth(), drawable.getIntrinsicHeight(), Bitmap.Config.ARGB_8888);
            final Canvas canvas = new Canvas(bmp);
            drawable.setBounds(0, 0, canvas.getWidth(), canvas.getHeight());
            drawable.draw(canvas);
            return bmp;
            /*
            Drawable backgroundDr = ((AdaptiveIconDrawable) drawable).getBackground();
            Drawable foregroundDr = ((AdaptiveIconDrawable) drawable).getForeground();

            Drawable[] drr = new Drawable[2];
            drr[0] = backgroundDr;
            drr[1] = foregroundDr;

            LayerDrawable layerDrawable = new LayerDrawable(drr);

            int width = layerDrawable.getIntrinsicWidth();
            int height = layerDrawable.getIntrinsicHeight();

            Bitmap bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888);

            Canvas canvas = new Canvas(bitmap);

            layerDrawable.setBounds(0, 0, canvas.getWidth(), canvas.getHeight());
            layerDrawable.draw(canvas);

            return bitmap;*/
        }

        return null;
    }
}
