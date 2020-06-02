package bitshift.qlaunch;

import android.content.Context;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.AdapterView;
import android.widget.BaseAdapter;
import android.widget.Filter;
import android.widget.Filterable;
import android.widget.GridView;
import android.widget.SearchView;
import android.widget.TextView;

import java.util.ArrayList;
import java.util.List;

/**
 * Created by fabian on 15/02/18.
 *
 * https://gist.github.com/DeepakRattan/26521c404ffd7071d0a4
 */

public class AllAppsAdapter extends BaseAdapter implements SearchView.OnQueryTextListener, Filterable, AdapterView.OnItemClickListener, AdapterView.OnItemLongClickListener {

    private Context mContext;
    private List<GroupItem> mList;
    private List<GroupItem> mStringFilterList;

    ValueFilter mValueFilter;

    private class ValueFilter extends Filter {
        @Override
        protected FilterResults performFiltering(CharSequence constraint) {
            FilterResults results = new FilterResults();

            if (constraint != null && constraint.length() > 0) {
                ArrayList<GroupItem> filterList = new ArrayList<GroupItem>();
                for (int i = 0; i < mStringFilterList.size(); i++) {
                    if ((mStringFilterList.get(i).label().toUpperCase())
                            .contains(constraint.toString().toUpperCase())) {

                        filterList.add(mStringFilterList.get(i));
                    }
                }
                results.count = filterList.size();
                results.values = filterList;
            } else {
                results.count = mStringFilterList.size();
                results.values = mStringFilterList;
            }
            return results;
        }

        @Override
        protected void publishResults(CharSequence constraint,
                                      FilterResults results) {
            mList = (ArrayList<GroupItem>) results.values;
            notifyDataSetChanged();
        }

    }

    AllAppsAdapter(Context context, List<GroupItem> list) {
        mList = list;
        mStringFilterList = mList;
        mContext = context;
    }

    @Override
    public int getCount() {
        return mList.size();
    }

    @Override
    public Object getItem(int i) {
        return mList.get(i);
    }

    @Override
    public long getItemId(int i) {
        return 0; //i;
    }

    @Override
    public View getView(int position, View view, ViewGroup parent) {
        GroupItem item = mList.get(position);
        return item.getView(mContext, view, parent);
    }

    @Override
    public Filter getFilter() {
        if (mValueFilter == null) {
            mValueFilter = new ValueFilter();
        }
        return mValueFilter;
    }

    @Override
    public boolean onQueryTextSubmit(String s) {
        getFilter().filter(s);
        return true;
    }

    @Override
    public boolean onQueryTextChange(String s) {
        getFilter().filter(s);
        return true;
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
}
