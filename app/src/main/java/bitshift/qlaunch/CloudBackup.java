package bitshift.qlaunch;

import android.app.backup.BackupAgentHelper;
import android.app.backup.BackupDataInput;
import android.app.backup.BackupDataOutput;
import android.app.backup.FileBackupHelper;
import android.app.backup.SharedPreferencesBackupHelper;
import android.os.ParcelFileDescriptor;
import android.util.Log;

import java.io.File;
import java.io.IOException;
import java.util.ArrayList;
import java.util.List;

/**
 * Created by fabian on 10/02/18.
 */

public class CloudBackup extends BackupAgentHelper {
    // The name of the SharedPreferences file
    static final String PREFS = "preferences";

    // A key to uniquely identify the set of backup data
    static final String PREFS_BACKUP_KEY = "preferences";

    static final String FILES_BACKUP_KEY = "files";

    @Override
    public void onCreate() {
        SharedPreferencesBackupHelper helper = new SharedPreferencesBackupHelper(this, PREFS);
        addHelper(PREFS_BACKUP_KEY, helper);

        List<File> groupList = getFileList(getApplicationContext().getFilesDir().toString() + "/data/group/");
        String[] stringList = new String[groupList.size()];
        int i = 0;
        for (File f : groupList)
        {
            stringList[i] = f.getAbsolutePath();
            ++i;
        }
        FileBackupHelper fileHelper = new FileBackupHelper(this, stringList);
        addHelper(FILES_BACKUP_KEY, fileHelper);
    }

    public List<File> getFileList(String path)
    {
        List<File> list = new ArrayList<File>();

        File[] files = new File(path).listFiles();
        for (File f : files)
        {
            if (!f.isDirectory() && f.exists())
                list.add(f);
        }
        return list;
    }

    @Override
    public void onBackup(ParcelFileDescriptor oldState, BackupDataOutput data, ParcelFileDescriptor newState) throws IOException
    {
        super.onBackup(oldState, data, newState);
        Log.d(Globals.APP_NAME, "Backing up");
    }

    @Override
    public void onRestore(BackupDataInput data, int appVersionCode, ParcelFileDescriptor newState) throws IOException
    {
        super.onRestore(data, appVersionCode, newState);
        Log.d(Globals.APP_NAME, "Backing up");
    }
}
