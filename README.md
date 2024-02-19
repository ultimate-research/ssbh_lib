# ssbh_lib
Libraries and tools for working with the SSBH binary formats in Rust.

## SSBH Formats
Click the links below to see the corresponding Rust source file with the file format's struct definitions.
All SSBH formats start with the file magic HBSS and use the same representation for types like arrays and offsets. 

| Format | Supported Versions (major.minor) |
| --- | --- |
| [Hlpb](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/hlpb.rs) (`.nuhlpb`) | 1.1 |
| [Matl](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/matl.rs) (`.numatb`) | 1.5, 1.6 |
| [Modl](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/modl.rs) (`.numdlb`,`.nusrcmdlb`) | 1.7 |
| [Mesh](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/mesh.rs) (`.numshb`) | 1.8, 1.9, 1.10 |
| [Skel](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/skel.rs) (`.nusktb`) | 1.0 |
| [Anim](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/anim.rs) (`.nuanmb`) | 1.2, 2.0, 2.1 |
| [Nlst](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/nlst.rs) (`.nulstb`) | 1.0 |
| [Nrpd](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/nrpd.rs) (`.nurpdb`) | 1.6 |
| [Nufx](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/nufx.rs) (`.nufxlb`) | 1.0, 1.1 |
| [Shdr](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/shdr.rs) (`.nushdb`) | 1.2 |

The ssbh_lib library also supports the non SSBH formats [MeshEx](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/meshex.rs) (`.numshexb`) and [Adj](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/adj.rs) (`.adjb`). 

If you find an SSBH format used in a game that isn't supported here, feel free to open a pull request to add it. The ssbh_lib types can also be used in your own project with the BinRead and SsbhWrite derives to support new SSBH formats without needing to modify ssbh_lib.

## Projects 
| Project | Description | Crate | Documentation |
| ---| ---| --- |--- |
| [ssbh_lib](https://github.com/ultimate-research/ssbh_lib/tree/master/ssbh_lib) | A library to read and write SSBH formats | [![Latest Version](https://img.shields.io/crates/v/ssbh_lib.svg)](https://crates.io/crates/ssbh_lib) |[![docs.rs](https://docs.rs/ssbh_lib/badge.svg)](https://docs.rs/ssbh_lib) |
| [ssbh_data](https://github.com/ultimate-research/ssbh_lib/tree/master/ssbh_data) | A high level API for reading and writing SSBH data | [![Latest Version](https://img.shields.io/crates/v/ssbh_data.svg)](https://crates.io/crates/ssbh_data) | [![docs.rs](https://docs.rs/ssbh_data/badge.svg)](https://docs.rs/ssbh_data) |

For making quick edits to SSBH files in a text editor, use [ssbh_lib_json](#ssbh_lib_json). [ssbh_data_json](#ssbh_data_json) supports fewer formats than ssbh_lib_json but adds the ability to decode and edit the buffer data in Mesh or Anim files. Python bindings for ssbh_data are available with [ssbh_data_py](https://github.com/ScanMountGoat/ssbh_data_py).

## Tools
- [ssbh_data_json](https://github.com/ultimate-research/ssbh_lib/tree/master/ssbh_data_json) - convert ssbh_data types to and from JSON
- [ssbh_lib_json](https://github.com/ultimate-research/ssbh_lib/tree/master/ssbh_lib_json) - convert ssbh_lib types to and from JSON
- [ssbh_test](https://github.com/ultimate-research/ssbh_lib/tree/master/ssbh_test) - test read/write for a game dump

## Building
With a recent version of Rust installed, run `cargo build --release`.

## Credits
- [SSBHLib](https://github.com/Ploaj/SSBHLib) - the original C# implementation for reading and writing SSBH files  
- [geometry_tools](https://github.com/ScanMountGoat/geometry_tools) - vertex data and geometry bounding calculations  
- [binrw](https://github.com/jam1garner/binrw) - binary parsing library and inspiration for porting the C# implementation to Rust  
- [glam](https://crates.io/crates/glam) - efficient vector and matrix math using SIMD
- *see the Cargo.toml files for the remaining projects used*
