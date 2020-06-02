package bitshift.qlaunch;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;

/**
 * Created by Bronson on 22/05/13.
 */
public class IntentActivity extends Activity
{
    public void onCreate(Bundle savedInstanceState)
    {
        super.onCreate(savedInstanceState);
        Intent intent = getIntent();
        IntentMgr.instance().onReceive(getApplicationContext(), intent);
        this.finish();
    }
}