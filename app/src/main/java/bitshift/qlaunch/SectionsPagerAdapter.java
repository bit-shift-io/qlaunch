// https://medium.com/@hanru.yeh/infinite-scrolling-horizontally-of-viewpager-or-uicollectionview-32755d0d2817

package bitshift.qlaunch;

import android.content.Context;
import android.content.res.Configuration;
import android.support.annotation.NonNull;
import android.support.v4.view.ViewPager;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.AdapterView;
import android.widget.GridView;

import java.util.List;

public class SectionsPagerAdapter extends CyclePagerAdapter<GroupItem> implements AdapterView.OnItemClickListener, AdapterView.OnItemLongClickListener, ViewPager.PageTransformer
{
    private static float ALPHA_SCALE = 1.75f; // 1 = default, 2 - fades quicker, 0 = fade slower
    private static float TRANSLATION_SCALE = 0.5f; // 0 = default, 1 = no scroll

    public SectionsPagerAdapter(@NonNull List<GroupItem> list, @NonNull CycleViewPager cycleViewPager) {
        super(list, cycleViewPager);
    }

    // Determines whether a page View is associated with a specific key object as returned by instantiateItem(ViewGroup, int).
    @Override
    public boolean isViewFromObject(View view, Object object)
    {
        return view.equals(object);
    }

    @Override
    public CharSequence getPageTitle(int position)
    {
        return mItemList.get(position).label();
    }

    @Override
    public Object instantiateItem(ViewGroup collection, int position)
    {
        LayoutInflater inflater = (LayoutInflater) collection.getContext().getSystemService(Context.LAYOUT_INFLATER_SERVICE);
        View view = inflater.inflate(R.layout.pager_view, null);

        // grid view
        GridView grid = view.findViewById(R.id.gridView);
        Group group = (Group) mItemList.get(position);

        GroupItemAdapter gridAdapter = new GroupItemAdapter(collection.getContext(), group, grid);
        grid.setAdapter(gridAdapter);
        grid.setOnItemClickListener(this); //click listener
        grid.setOnItemLongClickListener(this); //click listener

        collection.addView(view, 0);
        return view;
    }

    // Remove a page for the given position.
    @Override
    public void destroyItem(ViewGroup collection, int position, Object view)
    {
        collection.removeView((View) view);
    }

    @Override
    public void onItemClick(AdapterView<?> arg0, View view, int position, long id)
    {
        GridView mGrid = (GridView) arg0;
        GroupItem item = (GroupItem) mGrid.getAdapter().getItem(position);
        item.onItemClick(arg0, view, position, id);
    }

    @Override
    public boolean onItemLongClick(AdapterView<?> arg0, View view, int position, long id)
    {
        GridView mGrid = (GridView) arg0;
        GroupItem item = (GroupItem) mGrid.getAdapter().getItem(position);
        return item.onItemLongClick(arg0, view, position, id);
    }


    // TODO: getting some issues when you scroll a small distance and then the view becomes hidden - maybe use the scroll listener to "fix"
    //
    // handles fading between pages to help the app feel snappier
    public void transformPage(View view, float position)
    {
        /*
        int pageWidth = view.getWidth();
        if (position <= -1)
        {
            // -1 = left pane
            //view.setVisibility(View.GONE);
            view.setAlpha(0.0F);
        }
        else if (position >= 1)
        {
            // +1 = right pane
            //view.setVisibility(View.GONE);
            view.setAlpha(0.0F);
        }
        else
        {
            // [-1,1] 0 = visible page!
            float pos = Math.abs(position);
            float alpha = 1 - (pos * ALPHA_SCALE); // Alpha 1 = center, alpha 0 = edge

            if (alpha <= 0f)
                view.setVisibility(View.GONE);
            else
            {
                view.setVisibility(View.VISIBLE);
                view.setAlpha(alpha);

                // Counteract the default slide transition
                view.setTranslationX(pageWidth * -position * TRANSLATION_SCALE);
            }
        }
        */
    }

}