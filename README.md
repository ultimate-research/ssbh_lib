# ssbh_lib
Libraries and tools for working with the SSBH binary formats in Rust. The ssbh_lib library also serves as documentation for the SSBH format.
Report any bugs in any of these projects in [issues](https://github.com/ultimate-research/ssbh_lib/issues). See [Comparing two SSBH files](#Comparing-two-SSBH-files) for debugging tips to provide more useful feedback if a file isn't parsed or saved correctly. 

| Project | Description | Crate | Documentation |
| ---| ---| --- |--- |
| [ssbh_lib](https://github.com/ultimate-research/ssbh_lib/tree/master/ssbh_lib) | A library to parse and export SSBH formats | [![Latest Version](https://img.shields.io/crates/v/ssbh_lib.svg)](https://crates.io/crates/ssbh_lib) |[![docs.rs](https://docs.rs/ssbh_lib/badge.svg)](https://docs.rs/ssbh_lib) |
| [ssbh_data](https://github.com/ultimate-research/ssbh_lib/tree/master/ssbh_data) | A high level API for reading and writing SSBH data | [![Latest Version](https://img.shields.io/crates/v/ssbh_data.svg)](https://crates.io/crates/ssbh_data) | [![docs.rs](https://docs.rs/ssbh_data/badge.svg)](https://docs.rs/ssbh_data) |
| [ssbh_write_derive](https://github.com/ultimate-research/ssbh_lib/tree/master/ssbh_write_derive) | The derive macro for ssbh_lib |[![Latest Version](https://img.shields.io/crates/v/ssbh_write_derive.svg)](https://crates.io/crates/ssbh_write_derive) | [![docs.rs](https://docs.rs/ssbh_write_derive/badge.svg)](https://docs.rs/ssbh_write_derive) |

## SSBH Formats
Click the links below to see the corresponding Rust source filewith the file format's struct definitions. 
The main lib file for ssbh_lib contains shared parsing logic for arrays, enums, etc.  
| Format | Supported Versions (major.minor) |
| --- | --- |
| [Hlpb](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/hlpb.rs) (`.nuhlpb`) | 1.1 |
| [Matl](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/matl.rs) (`.numatb`) | 1.6 |
| [Modl](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/modl.rs) (`.numdlb`,`.nusrcmdlb`) | 1.7 |
| [Mesh](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/mesh.rs) (`.numshb`) | 1.8, 1.9, 1.10 |
| [Skel](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/skel.rs) (`.nusktb`) | 1.0 |
| [Anim](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/anim.rs) (`.nuanmb`) | 1.2, 2.0, 2.1 |
| [Nrpd](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/nrpd.rs) (`.nurpdb`) | 1.6 |
| [Nufx](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/nufx.rs) (`.nufxlb`) | 1.0, 1.1 |
| [Shdr](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/shdr.rs) (`.nushdb`) | 1.2 |

The ssbh_lib library also supports the non SSBH formats [MeshEx](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/meshex.rs) (`.numshexb`) and [Adj](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/adj.rs) (`.adjb`).  

## ssbh_lib_json
A command line tool for creating and editing SSBH binary data using JSON. The MeshEx and Adj formats are also supported. Drag a properly formatted JSON file onto the executable to create a binary file. Drag a supported file format onto the executable to create a JSON file. Byte arrays are encoded as hex strings for SSBH types. JSON files are text files, so they can be viewed and edited in any text editor such as [VSCode](https://code.visualstudio.com/).

### Usage
A prebuilt binary for Windows is available in [releases](https://github.com/ultimate-research/ssbh_lib/releases).  
`ssbh_lib_json.exe <input>`  
`ssbh_lib_json.exe <input> <output>`  

### Editing a binary file
- Output the JSON with `ssbh_lib_json.exe model.numshb mesh.json`  
- Make changes to the JSON file such as adding elements to an array or changing field values
- Save the changes to a new file with `ssbh_lib_json.exe mesh.json model.new.numshb`

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

## Credits
- [SSBHLib](https://github.com/Ploaj/SSBHLib) - the original C# implementation for reading and writing SSBH files  
- [geometry_tools](https://github.com/ScanMountGoat/geometry_tools) - vertex data and geometry bounding calculations  
- [BinRead](https://crates.io/crates/binread) - binary parsing library and inspiration for porting the C# implementation to Rust  
- [glam](https://crates.io/crates/glam) - efficient vector and matrix math using SIMD
- *see the cargo.toml files for the remaining projects used*
