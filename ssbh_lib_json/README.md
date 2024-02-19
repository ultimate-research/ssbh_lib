
# ssbh_lib_json
A command-line tool for creating and editing SSBH binary data using JSON. The MeshEx and Adj formats are also supported. Drag a properly formatted JSON file onto the executable to create a binary file. Drag a supported file format onto the executable to create a JSON file. Byte arrays are encoded as hex strings for SSBH types. JSON files are text files, so they can be viewed and edited in any text editor such as [VSCode](https://code.visualstudio.com/).

Sample output from an Hlpb file.

```json
{
  "Hlpb": {
    "V11": {
      "aim_constraints": [],
      "orient_constraints": [
        {
          "name": "nuHelperBoneRotateInterp265",
          "parent_bone_name1": "ArmL",
          "parent_bone_name2": "ArmL",
          "source_bone_name": "HandL",
          "target_bone_name": "H_WristL",
          "unk_type": 2,
          "constraint_axes": {
            "x": 0.5,
            "y": 0.5,
            "z": 0.5
          },
          "quat1": {
            "x": 0.707107,
            "y": 0.0,
            "z": 0.0,
            "w": 0.707107
          },
          "quat2": {
            "x": -0.707107,
            "y": 0.0,
            "z": 0.0,
            "w": 0.707107
          },
          "range_min": {
            "x": -180.0,
            "y": -180.0,
            "z": -180.0
          },
          "range_max": {
            "x": 180.0,
            "y": 180.0,
            "z": 180.0
          }
        }
      ],
      "constraint_indices": [0],
      "constraint_types": ["Orient"]
    }
  }
}
```

### Usage
A prebuilt binary for Windows is available in [releases](https://github.com/ultimate-research/ssbh_lib/releases).  
`ssbh_lib_json.exe <input>`  
`ssbh_lib_json.exe <input> <output>`  

### Editing a binary file
- Output the JSON with `ssbh_lib_json.exe model.numshb mesh.json`  
- Make changes to the JSON file such as adding elements to an array or changing field values
- Save the changes to a new file with `ssbh_lib_json.exe mesh.json model_new.numshb`

### Comparing two binary files
ssbh_lib_json is used frequently during the development of ssbh_lib and ssbh_data for determining changes to a file without manually inspecting the file in a hex editor. 
- Output the JSON for both files with `ssbh_lib_json.exe matl1.numatb matl1.json` and `ssbh_lib_json.exe matl2.numatb matl2.json` 
- Compare the text output for both JSON files to see changes, additions, and deletions to the data stored file using a diffing tool or [diff using VSCode](https://vscode.one/diff-vscode/).

Comparing the binary and JSON representations of two files gives clues as to how and why the binary files differ. 
| JSON Identical | Binary Identical | Conclusion |
| --- | --- | --- |
| :x: | :x: | The two files do not contain the same data or the struct definitions do not capture all the data in the given file format. |
| :heavy_check_mark: | :x: | The files differ in padding or alignment but contain the same data, or fields are missing from the type definitions. |
| :heavy_check_mark: | :heavy_check_mark: | The files are identical and contain the same data |
