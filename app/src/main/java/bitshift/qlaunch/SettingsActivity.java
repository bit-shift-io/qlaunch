package bitshift.qlaunch;

import android.content.Intent;
import android.os.Bundle;
import android.preference.Preference;
import android.preference.PreferenceActivity;
import android.preference.PreferenceFragment;


// settings activity
// https://alvinalexander.com/android/android-tutorial-preferencescreen-preferenceactivity-preferencefragment
public class SettingsActivity extends PreferenceActivity
{
	// ON CREATE
	@Override
	public void onCreate(Bundle savedInstanceState) 
	{
		super.onCreate(savedInstanceState);
		getFragmentManager().beginTransaction().replace(android.R.id.content, new SettingsFragment()).commit();
	}

	public static class SettingsFragment extends PreferenceFragment
	{
		@Override
		public void onCreate(final Bundle savedInstanceState)
		{
			super.onCreate(savedInstanceState);
			addPreferencesFromResource(R.xml.preferences);

			connectButton("key_factory_reset_button");
			connectButton("key_restart_launcher");
		}

		void connectButton(String key) {
			Preference button = findPreference(key);

			class PreferenceClickListener implements Preference.OnPreferenceClickListener {
				SettingsFragment mSettingsFragment;

				PreferenceClickListener(SettingsFragment settingsFragment) {
					mSettingsFragment = settingsFragment;
				}

				@Override
				public boolean onPreferenceClick(Preference preference) {
					//code for what you want it to do
					MainActivity.instance().onPreferenceClick(mSettingsFragment, preference);
					return true;
				}
			}
			button.setOnPreferenceClickListener(new PreferenceClickListener(this));
		}

		void close() {
			//Intent result = new Intent();
			//result.putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, mAppWidgetId);
			//getActivity().setResult(RESULT_OK, result);
			getActivity().finish();
		}
	}

}
