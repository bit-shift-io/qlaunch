package bitshift.qlaunch;

import android.content.Context;
import android.graphics.Canvas;
import android.support.v4.view.PagerTabStrip;
import android.util.AttributeSet;
import android.util.Log;

import java.lang.reflect.Field;

/**
 * Created by fabian on 10/02/18.
 */

public class PagerTabStripNoInd extends PagerTabStrip {
    private static final String TAG = "bitshift.qlaunch.PagerTabStripNoInd";
    private static final Integer ZERO = new Integer(0);
    public PagerTabStripNoInd(Context context) {
        super(context);
    }
    public PagerTabStripNoInd(Context context, AttributeSet attrs) {
        super(context, attrs);
    }

    @Override
    protected void onDraw(Canvas canvas) {
        /*
        try {
            Field tabAlphaField = PagerTabStrip.class.getField("mTabAlpha");
            tabAlphaField.setAccessible(true);
            tabAlphaField.set(this, ZERO);
        } catch (NoSuchFieldException e) {
            Log.e(TAG, "onDraw", e);
        } catch (IllegalAccessException e) {
            Log.e(TAG, "onDraw", e);
        }
        super.onDraw(canvas);*/
    }
}
