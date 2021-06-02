# ssbh_lib
An SSBH parsing library in Rust. Each SSBH format has a major and minor version. All versions used by Smash Ultimate are supported. This library also serves as documentation for the SSBH format. Report any bugs in any of these projects in [issues](https://github.com/ultimate-research/ssbh_lib/issues). See [Comparing two SSBH files](#Comparing-two-SSBH-files) for debugging tips to provide more useful feedback if a file isn't parsed or saved correctly. 

## SSBH Formats
Click the links below to see the corresponding Rust source filewith the file format's struct definitions. 
The main lib file for ssbh_lib contains shared parsing logic for arrays, enums, etc.  
| Format | Supported Versions (major.minor) |
| --- | --- |
| [Hlpb](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/hlpb.rs) (`.nuhlpb`) | 1.1 |
| [Matl](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/matl.rs) (`.numatb`) | 1.6 |
| [Modl](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/modl.rs) (`.numdlb`,`.nusrcmdlb`) | 1.7 |
| [Mesh](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/mesh.rs) (`.numshb`) | 1.8, 1.10 |
| [Skel](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/skel.rs) (`.nusktb`) | 1.0 |
| [Anim](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/anim.rs) (`.nuanmb`) | 2.0, 2.1 |
| [Nrpd](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/nrpd.rs) (`.nurpdb`) | 1.6 |
| [Nufx](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/nufx.rs) (`.nufxlb`) | 1.0, 1.1 |
| [Shdr](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/shdr.rs) (`.nushdb`) | 1.2 |

Non SSBH Formats:
* [MeshEx](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/meshex.rs) (`.numshexb`)
* [Adj](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/adj.rs) (`.adjb`)

# ssbh_data
A higher level data access layer for some SSBH formats. Python bindings are available with [ssbh_data_py](https://github.com/ScanMountGoat/ssbh_data_py). 

# ssbh_lib_json
A program for creating and editing SSBH binary data using JSON. Drag a properly formatted JSON file onto the executable to create a binary file. Drag a supported file format onto the executable to create a JSON file. Byte arrays are encoded as hex strings. JSON files are text files, so they can be viewed and edited in any text editor such as [VSCode](https://code.visualstudio.com/).

## Usage
A prebuilt binary for Windows is available in [releases](https://github.com/ultimate-research/ssbh_lib/releases).  
`ssbh_lib_json.exe <input>`  
`ssbh_lib_json.exe <input> <output>`  

### Editing an SSBH file
- Output the JSON with `ssbh_lib_json.exe model.numshb mesh.json`  
- Make changes to the JSON file such as adding elements to an array or changing field values
- Save the changes to a new file with `ssbh_lib_json.exe mesh.json model.new.numshb`

### Comparing two SSBH files
ssbh_lib_json is used frequently during the development of ssbh_lib and ssbh_data for determining changes to a file without manually inspecting the file in a hex editor. 
- Output the JSON for both files with `ssbh_lib_json.exe matl1.numatb matl1.json` and `ssbh_lib_json.exe matl2.numatb matl2.json` 
- Compare the text output for both JSON files to see changes, additions, and deletions to the data stored file using a diffing tool or [diff using VSCode](https://vscode.one/diff-vscode/).

Comparing the binary and JSON representations of two files gives clues as to how and why the binary files differ. 
| JSON Identical | Binary Identical | Conclusion |
| --- | --- | --- |
| :x: | :x: | The two files do not contain the same data or the struct definitions do not capture all the data in the given file format. |
| :heavy_check_mark: | :x: | The files differ in padding or alignment but contain the same data. |
| :heavy_check_mark: | :heavy_check_mark: | The files are identical and contain the same data |

# Credits
The original C# implementation can be found in the [SSBHLib](https://github.com/Ploaj/SSBHLib) repo.
