// https://medium.com/@hanru.yeh/infinite-scrolling-horizontally-of-viewpager-or-uicollectionview-32755d0d2817

package bitshift.qlaunch;

import android.content.Context;
import android.support.annotation.NonNull;
import android.support.v4.view.PagerAdapter;
import android.support.v4.view.ViewPager;
import android.util.AttributeSet;

public class CycleViewPager extends ViewPager {

    public CycleViewPager(Context context) {
        super(context);
    }

    public CycleViewPager(Context context, AttributeSet attrs) {
        super(context, attrs);
    }

    @Override
    public void setAdapter(@NonNull PagerAdapter adapter) {
        super.setAdapter(adapter);
        // after setting adapter, maybe scroll to 2nd itemview
        if (adapter.getCount() > 1) {
            setCurrentItem(1, false);
        }
    }
}