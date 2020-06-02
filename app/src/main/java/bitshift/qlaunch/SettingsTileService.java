package bitshift.qlaunch;

import android.content.Intent;
import android.content.res.Resources;
import android.service.quicksettings.TileService;
import android.util.Log;

/**
 * Created by fabian on 9/02/18.
 *
 * Put settings as a Quick Launch Tile
 *
 * https://github.com/googlecodelabs/android-n-quick-settings/search?utf8=%E2%9C%93&q=TileService&type=
 */

public class SettingsTileService extends TileService {


    @Override
    public void onTileAdded()
    {
        Log.d(Globals.APP_NAME, "Tile added");
        super.onTileAdded();
    }

    @Override
    public void onTileRemoved()
    {
        Log.d(Globals.APP_NAME, "tile removed");
        super.onTileRemoved();
    }

    @Override
    public void onStartListening()
    {
        Log.d(Globals.APP_NAME, "start settings service");
        super.onStartListening();
    }

    @Override
    public void onStopListening()
    {
        Log.d(Globals.APP_NAME, "service stopped");
        super.onStopListening();
    }

    @Override
    public void onClick()
    {
        Log.d(Globals.APP_NAME, "click");
        super.onClick();

        // Check to see if the device is currently locked.
        boolean isCurrentlyLocked = this.isLocked();

        if (!isCurrentlyLocked) {

            Resources resources = getApplication().getResources();

            //Tile tile = getQsTile();
            //String tileLabel = tile.getLabel().toString();
            //String tileState = (tile.getState() == Tile.STATE_ACTIVE) ? resources.getString(R.string.service_active) : resources.getString(R.string.service_inactive);

            Intent intent = new Intent(getApplicationContext(),
                    SettingsActivity.class);

            //intent.putExtra(ResultActivity.RESULT_ACTIVITY_NAME_KEY, tileLabel);
            //intent.putExtra(ResultActivity.RESULT_ACTIVITY_INFO_KEY, tileState);

            startActivityAndCollapse(intent);
        }
    }
}
