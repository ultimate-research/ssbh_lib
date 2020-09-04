# ssbh_lib
A parsing library for SSBH formats in Rust. Parsing is implemented for the following types:  
* Hlpb (`.nuhlpb`)
* Matl (`.numatb`)
* Modl (`.numdlb`)
* Mesh (`.numatb`)
* Skel (`.nusktb`)
* Anim (`.nuanmb`)

# Usage
The binary application prints any supported file to the console in JSON format.  
`ssbh_lib_json.exe <path to SSBH>`  
Example for saving the output to a file:  
`ssbh_lib_json.exe "model.numatb" > output.json`

# Credits
The original C# file formats and parsing code can be found in the [SSBHLib](https://github.com/Ploaj/SSBHLib) repo.
