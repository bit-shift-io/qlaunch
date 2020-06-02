// https://medium.com/@hanru.yeh/infinite-scrolling-horizontally-of-viewpager-or-uicollectionview-32755d0d2817

package bitshift.qlaunch;

import android.support.annotation.CallSuper;
import android.support.annotation.IntRange;
import android.support.annotation.NonNull;
import android.support.v4.view.PagerAdapter;
import android.support.v4.view.ViewPager;

import java.lang.ref.WeakReference;
import java.util.ArrayList;
import java.util.List;

public abstract class CyclePagerAdapter<T> extends PagerAdapter implements ViewPager.OnPageChangeListener {
    protected List<T> mItemList = null;
    private WeakReference<CycleViewPager> mViewPagerRef = null;

    public CyclePagerAdapter(@NonNull List<T> list, @NonNull CycleViewPager cycleViewPager) {
        setItemList(list);
        cycleViewPager.addOnPageChangeListener(this);
        mViewPagerRef = new WeakReference<>(cycleViewPager);
    }

    public void setItemList(@NonNull List<T> list) {
        if (list.size() <= 1) {
            // needn't to support infinite cycle
            mItemList = new ArrayList<>(list);
        } else {
            // refer: https://maniacdev.com/2013/08/tutorial-how-to-create-an-infinite-scrolling-uicollectionview
            mItemList = new ArrayList<>(getLengthSafe(list) + 2);
            mItemList.add(list.get(getLengthSafe(list) - 1));
            mItemList.addAll(list);
            mItemList.add(list.get(0));
        }

        notifyDataSetChanged();
    }

    private int getLengthSafe(List<T> list)
    {
        if (list == null)
            return 0;

        return list.size();
    }

    public boolean isCycleEnable() {
        return getCount() > 1;
    }

    protected boolean isFakePosition(@IntRange(from = 0) int position) {
        if (isCycleEnable()) {
            return position == 0 || position == getLengthSafe(mItemList) - 1;
        }
        return false;
    }

    public int getCyclePosition(@IntRange(from = 0) int actualPosition) {
        int cyclePosition = actualPosition;
        if (isCycleEnable()) {
            cyclePosition = (cyclePosition - 1) % (getCount() - 2);
        }
        return cyclePosition;
    }

    @Override
    public int getCount() {
        return getLengthSafe(mItemList);
    }

    @CallSuper
    @Override
    public void onPageScrolled(int position, float positionOffset, int positionOffsetPixels) {
        // no operation
    }

    @CallSuper
    @Override
    public void onPageSelected(int position) {
        // no operation
    }

    @CallSuper
    @Override
    public void onPageScrollStateChanged(int state) {
        if (mViewPagerRef == null || mViewPagerRef.get() == null || !isCycleEnable()) {
            return;
        }

        if (state == ViewPager.SCROLL_STATE_IDLE) {

            CycleViewPager viewPager = mViewPagerRef.get();
            if (viewPager.getCurrentItem() == getCount() - 1) {
                viewPager.post(new Runnable() {
                    @Override
                    public void run() {
                        if (mViewPagerRef == null || mViewPagerRef.get() == null) {
                            return;
                        }
                        ViewPager viewPager = mViewPagerRef.get();
                        if (viewPager.getCurrentItem() == getCount() - 1) {
                            viewPager.setCurrentItem(1, false);
                        }
                    }
                });
            } else if (viewPager.getCurrentItem() == 0) {
                viewPager.post(new Runnable() {
                    @Override
                    public void run() {
                        if (mViewPagerRef == null || mViewPagerRef.get() == null) {
                            return;
                        }
                        ViewPager viewPager = mViewPagerRef.get();
                        if (viewPager.getCurrentItem() == 0) {
                            viewPager.setCurrentItem(getCount() - 2, false);
                        }
                    }
                });
            }
        }
    }
}