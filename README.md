# ssbh_lib
Libraries and tools for working with the SSBH binary formats in Rust.

## SSBH Formats
Click the links below to see the corresponding Rust source file with the file format's struct definitions.

| Format | Supported Versions (major.minor) |
| --- | --- |
| [Hlpb](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/hlpb.rs) (`.nuhlpb`) | 1.1 |
| [Matl](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/matl.rs) (`.numatb`) | 1.5, 1.6 |
| [Modl](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/modl.rs) (`.numdlb`,`.nusrcmdlb`) | 1.7 |
| [Mesh](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/mesh.rs) (`.numshb`) | 1.8, 1.9, 1.10 |
| [Skel](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/skel.rs) (`.nusktb`) | 1.0 |
| [Anim](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/anim.rs) (`.nuanmb`) | 1.2, 2.0, 2.1 |
| [Nrpd](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/nrpd.rs) (`.nurpdb`) | 1.6 |
| [Nufx](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/nufx.rs) (`.nufxlb`) | 1.0, 1.1 |
| [Shdr](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/shdr.rs) (`.nushdb`) | 1.2 |

The ssbh_lib library also supports the non SSBH formats [MeshEx](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/meshex.rs) (`.numshexb`) and [Adj](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/adj.rs) (`.adjb`).  

## Projects 
| Project | Description | Crate | Documentation |
| ---| ---| --- |--- |
| [ssbh_lib](https://github.com/ultimate-research/ssbh_lib/tree/master/ssbh_lib) | A library to read and write SSBH formats | [![Latest Version](https://img.shields.io/crates/v/ssbh_lib.svg)](https://crates.io/crates/ssbh_lib) |[![docs.rs](https://docs.rs/ssbh_lib/badge.svg)](https://docs.rs/ssbh_lib) |
| [ssbh_data](https://github.com/ultimate-research/ssbh_lib/tree/master/ssbh_data) | A high level API for reading and writing SSBH data | [![Latest Version](https://img.shields.io/crates/v/ssbh_data.svg)](https://crates.io/crates/ssbh_data) | [![docs.rs](https://docs.rs/ssbh_data/badge.svg)](https://docs.rs/ssbh_data) |

For making quick edits to SSBH files in a text editor, use [ssbh_lib_json](#ssbh_lib_json). [ssbh_data_json](#ssbh_data_json) supports fewer formats than ssbh_lib_json but adds the ability to decode and edit the buffer data in Mesh or Anim files. Python bindings for ssbh_data are available with [ssbh_data_py](https://github.com/ScanMountGoat/ssbh_data_py). 

## ssbh_lib_json
A command-line tool for creating and editing SSBH binary data using JSON. The MeshEx and Adj formats are also supported. Drag a properly formatted JSON file onto the executable to create a binary file. Drag a supported file format onto the executable to create a JSON file. Byte arrays are encoded as hex strings for SSBH types. JSON files are text files, so they can be viewed and edited in any text editor such as [VSCode](https://code.visualstudio.com/).

Sample output from a portion of an Hlpb file.

```json
{
  "Hlpb": {
    "V11": {
      "aim_constraints": [],
      "orient_constraints": [
        {
          "name": "nuHelperBoneRotateInterp339",
          "parent_bone_name1": "ArmL",
          "parent_bone_name2": "ArmL",
          "source_bone_name": "HandL",
          "target_bone_name": "H_WristL",
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

## ssbh_data_json
A command-line tool for creating and editing SSBH binary data using JSON. Drag a properly formatted JSON file onto the executable to create a binary file. Drag a supported file format onto the executable to create a JSON file.

Sample output from a portion of an Anim file.

```json
"name": "CustomVector8",
"scale_options": {
  "inherit_scale": false,
  "compensate_scale": false
},
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

## Building
With Rust 1.60 or later installed, run `cargo build --release`.

## Credits
- [SSBHLib](https://github.com/Ploaj/SSBHLib) - the original C# implementation for reading and writing SSBH files  
- [geometry_tools](https://github.com/ScanMountGoat/geometry_tools) - vertex data and geometry bounding calculations  
- [BinRead](https://crates.io/crates/binread) - binary parsing library and inspiration for porting the C# implementation to Rust  
- [glam](https://crates.io/crates/glam) - efficient vector and matrix math using SIMD
- *see the Cargo.toml files for the remaining projects used*
