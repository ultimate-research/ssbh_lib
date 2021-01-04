# ssbh_lib
A parsing library for SSBH formats in Rust. Each SSBH format has a major and minor version. Only the versions used by Smash Ultimate are supported. Parsing is implemented for the following types:  
* Hlpb (`.nuhlpb`)
* Matl (`.numatb`)
* Modl (`.numdlb`)
* Mesh (`.numatb`)
* Skel (`.nusktb`)
* Anim (`.nuanmb`)
* Nrpd (`nurpdb`)
* Nufx (`nufxlb`)

# Usage
The binary application exports any supported file to JSON format.  
If no output is specified, the output file will be the input with `.json` appended.  
`ssbh_lib_json.exe <ssbh file>`  
`ssbh_lib_json.exe <ssbh file> <json output>`  

# Credits
The original C# file formats and parsing code can be found in the [SSBHLib](https://github.com/Ploaj/SSBHLib) repo.
