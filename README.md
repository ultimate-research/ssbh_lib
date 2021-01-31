# ssbh_lib
An SSBH parsing library in Rust. Each SSBH format has a major and minor version. Only some of the versions used by Smash Ultimate are supported. This library also serves as documentation for the SSBH format. Click the links below to see the corresponding Rust source file in `src/formats` with the file formats struct definitions. The `src/lib.rs` file contains shared parsing logic for arrays, enums, etc.  
* [Hlpb](https://github.com/ultimate-research/ssbh_lib/blob/master/src/formats/hlpb.rs) (`.nuhlpb`)
* [Matl](https://github.com/ultimate-research/ssbh_lib/blob/master/src/formats/matl.rs) (`.numatb`)
* [Modl](https://github.com/ultimate-research/ssbh_lib/blob/master/src/formats/modl.rs) (`.numdlb`)
* [Mesh](https://github.com/ultimate-research/ssbh_lib/blob/master/src/formats/mesh.rs) (`.numatb`)
* [Skel](https://github.com/ultimate-research/ssbh_lib/blob/master/src/formats/skel.rs) (`.nusktb`)
* [Anim](https://github.com/ultimate-research/ssbh_lib/blob/master/src/formats/anim.rs) (`.nuanmb`)
* [Nrpd](https://github.com/ultimate-research/ssbh_lib/blob/master/src/formats/nrpd.rs) (`.nurpdb`)
* [Nufx](https://github.com/ultimate-research/ssbh_lib/blob/master/src/formats/nufx.rs) (`.nufxlb`)
* [MeshEx](https://github.com/ultimate-research/ssbh_lib/blob/master/src/formats/meshex.rs) (`.numshexb`)

# ssbh_lib_json
The binary application exports any supported file to JSON format. If no output is specified, the output file will be the input with `.json` appended. This also allows dragging a supported file format onto the executable to extract it to JSON. Byte arrays are encoded as hex strings.  

`ssbh_lib_json.exe <ssbh file>`  
`ssbh_lib_json.exe <ssbh file> <json output>`  

# Credits
The original C# file formats and parsing code can be found in the [SSBHLib](https://github.com/Ploaj/SSBHLib) repo.
