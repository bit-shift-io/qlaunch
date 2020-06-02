package bitshift.qlaunch;

import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.support.v4.app.DialogFragment;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.view.Window;
import android.widget.AdapterView;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.EditText;
import android.widget.ImageView;
import android.widget.Spinner;
import android.widget.TextView;

import java.util.ArrayList;
import java.util.List;

/**
 * Created by fabian on 9/02/18.
 *
 * Dialog used to edit the application settings
 */

public class ApplicationSettingsDialog extends DialogFragment {

    Application mApp;
    AdapterView<?> mAdapterView;

    static ApplicationSettingsDialog newInstance(Application app, AdapterView<?> adapterView) {
        ApplicationSettingsDialog f = new ApplicationSettingsDialog();
        f.mApp = app;
        f.mAdapterView = adapterView;
        return f;
    }

    @Override
    public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
    }

    @Override
    public View onCreateView(LayoutInflater inflater, ViewGroup container,
                             Bundle savedInstanceState) {

        getDialog().getWindow().requestFeature(Window.FEATURE_NO_TITLE);

        View dialogView = inflater.inflate(R.layout.dialog_application_settings, container, false);

        // now populate the builder(final allows us to use it in the onclick methods
        final EditText appLabel = dialogView.findViewById(R.id.text_app_label);
        appLabel.setText(mApp.label());

        final TextView categoryLabel = dialogView.findViewById(R.id.category_label);
        if (mApp.category() != null && !mApp.category().isEmpty())
            categoryLabel.setText(mApp.category());
        else
            categoryLabel.setVisibility(View.GONE);

        final ImageView imgIcon = dialogView.findViewById(R.id.image_app_icon);
        imgIcon.setImageDrawable(mApp.applicationIcon());
        imgIcon.setOnClickListener(new View.OnClickListener()
        {
            @Override
            public void onClick(View view)
            {
                String select = mAdapterView.getContext().getResources().getString(R.string.dialog_title_select_image);

                Intent intent = new Intent(Intent.ACTION_GET_CONTENT);
                intent.setType("image/*");
                mAdapterView.getContext().startActivity(Intent.createChooser(intent, select));

                /*
                Intent imageIntent = new Intent(Intent.ACTION_PICK, android.provider.MediaStore.Images.Media.EXTERNAL_CONTENT_URI);
                adapView.getContext().startActivity(imageIntent);
                */
            }
        });

		final CheckBox checkUnintstall = dialogView.findViewById(R.id.check_uninstall);
		final String packageName = mApp.packageName();

		// setp the spinner box
		final Spinner spinGroup = dialogView.findViewById(R.id.spinner_app_group);
		final Group rootGroup = PackageMgr.instance().rootGroup();

		List<String> spinList = new ArrayList<String>(); //make list of groups
        final List<GroupItem> childList = rootGroup.childList(GroupItem.FLAG_ALL);

		for (GroupItem item : rootGroup.childList(GroupItem.FLAG_ALL))
			spinList.add(item.label());

		ArrayAdapter<String> spinAdapter = new ArrayAdapter<String>(mAdapterView.getContext(), android.R.layout.simple_list_item_1, spinList);
		spinGroup.setAdapter(spinAdapter);

		for (int i = 0; i < childList.size(); ++i) // set our current group as selection
			if (mApp.parent().equals(childList.get(i)))
			{
				spinGroup.setSelection(i);
				break;
			}


        // Watch for button clicks.
        Button okBtn = dialogView.findViewById(R.id.ok_button);
        okBtn.setOnClickListener(new View.OnClickListener() {
            public void onClick(View v) {
                if (checkUnintstall.isChecked()) // UNINSTALL
                {
                    Uri uninsUri = Uri.parse(String.format("package:%s", packageName));
                    Intent uninstallIntent = new Intent(Intent.ACTION_DELETE, uninsUri);
                    uninstallIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                    mAdapterView.getContext().startActivity(uninstallIntent);
                    // the receiver will then determine if this package was removed :)
                }
                else // update this package
                {
                    mApp.setLabel(appLabel.getText().toString());
                    mApp.setLabel(); // make sure its not blank

                    int pos = spinGroup.getSelectedItemPosition();

                    Group oldParent = mApp.parent();
                    oldParent.remove(mApp);

                    Group newParent = (Group) childList.get(pos);
                    newParent.add(mApp);

                    mApp.update(); // update newParent
                }

                // write changes for this app to file
                mApp.writeToFile();

                dismiss();
            }
        });

        Button cancelBtn = dialogView.findViewById(R.id.cancel_button);
        cancelBtn.setOnClickListener(new View.OnClickListener() {
             public void onClick(View v) {
                dismiss();
             }
         });


/*
	    builder.setPositiveButton(R.string.apply, new DialogInterface.OnClickListener()
	    {
            @Override
            public void onClick(DialogInterface dialog, int id)
            {
            	if (checkUnintstall.isChecked()) // UNINSTALL
            	{
            		Uri uninsUri = Uri.parse(String.format("package:%s", packageName));
            		Intent uninstallIntent = new Intent(Intent.ACTION_DELETE, uninsUri);
            		uninstallIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            		adapView.getContext().startActivity(uninstallIntent);
            		// the receiver will then determine if this package was removed :)
            	}
            	else // update this package
            	{
                    thisApp.setLabel(appLabel.getText().toString());
                    thisApp.setLabel(); // make sure its not blank

            		int pos = spinGroup.getSelectedItemPosition();

            		Group oldParent = (Group) thisApp.parent();
                    oldParent.remove(thisApp);

            		Group newParent = (Group) childList.get(pos);
                    newParent.add(thisApp);

            		thisApp.update(); // update newParent
            	}

                // write changes for this app to file
                writeToFile();
            }
        });
        builder.setNegativeButton(R.string.cancel, null);

        AlertDialog dialog = builder.create();
        dialog.show();
        */


        /*
        View tv = v.findViewById(R.id.text);
        ((TextView)tv).setText("Dialog #" + mNum + ": using style "
                + getNameForNum(mNum));
/*
        // Watch for button clicks.
        Button button = (Button)v.findViewById(R.id.show);
        button.setOnClickListener(new View.OnClickListener() {
            public void onClick(View v) {
                // When button is clicked, call up to owning activity.
                ((FragmentDialog)getActivity()).showDialog();
            }
        });*/

        return dialogView;
    }

}