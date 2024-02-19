
# ssbh_data_json
A command-line tool for creating and editing SSBH binary data using JSON. Drag a properly formatted JSON file onto the executable to create a binary file. Drag a supported file format onto the executable to create a JSON file.

Sample output from a TransformTrack in an Anim file.

```json
{
  "name": "CustomVector8",
  "compensate_scale": false,
  "transform_flags": {
    "override_translation": false,
    "override_rotation": false,
    "override_scale": false
  },
  "values": {
    "Vector4": [
      {
        "x": 1.0,
        "y": 1.0,
        "z": 1.0,
        "w": 1.0
      }
    ]
  }
}
```

### Feature Comparison
 ssbh_data_json provides a simplified and more readable output compared to ssbh_lib_json. This means that resaving a file with ssbh_data_json may result in a file that is not binary identical with the original since some data needs to be recalculated.

| feature | ssbh_lib_json | ssbh_data_json |
| --- | --- | --- |
| Convert SSBH files to and from JSON | :heavy_check_mark: | :heavy_check_mark: |
| Mesh and Skel Buffer encoding/decoding | :x: | :heavy_check_mark: |
| Rebuild binary identical output | :heavy_check_mark: | :x: |
| Resave SSBH files as a different version | :x: | :heavy_check_mark: |

### Usage
`ssbh_data_json.exe <input>`  
`ssbh_data_json.exe <input> <output>`  

### Editing a binary file
- Output the JSON with `ssbh_lib_json.exe model.numshb mesh.json`  
- Make changes to the JSON file such as adding elements to an array or changing field values
- Save the changes to a new file with `ssbh_lib_json.exe mesh.json model_new.numshb`
