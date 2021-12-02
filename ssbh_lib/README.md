# ssbh_lib [![Latest Version](https://img.shields.io/crates/v/ssbh_lib.svg)](https://crates.io/crates/ssbh_lib) [![docs.rs](https://docs.rs/ssbh_lib/badge.svg)](https://docs.rs/ssbh_lib)  
An SSBH parsing and exporting library in Rust. Each SSBH format has a major and minor version. All versions used by Smash Ultimate are supported.  

| Format | Supported Versions (major.minor) |
| --- | --- |
| [Hlpb](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/hlpb.rs) (`.nuhlpb`) | 1.1 |
| [Matl](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/matl.rs) (`.numatb`) | 1.6 |
| [Modl](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/modl.rs) (`.numdlb`,`.nusrcmdlb`) | 1.7 |
| [Mesh](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/mesh.rs) (`.numshb`) | 1.8, 1.10 |
| [Skel](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/skel.rs) (`.nusktb`) | 1.0 |
| [Anim](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/anim.rs) (`.nuanmb`) | 1.2, 2.0, 2.1 |
| [Nrpd](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/nrpd.rs) (`.nurpdb`) | 1.6 |
| [Nufx](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/nufx.rs) (`.nufxlb`) | 1.0, 1.1 |
| [Shdr](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_lib/src/formats/shdr.rs) (`.nushdb`) | 1.2 |


## Example
A traditional struct definition for SSBH data may look like the following.
```rust
struct FileData {
    name: u64,
    name_offset: u64,
    values_offset: u64,
    values_count: u64
}
```
The `FileData` struct has the correct size to represent the data on disk but has a number of issues.
The `values` array doesn't capture the fact that SSBH arrays are strongly typed.
It's not clear if the `name_offset` is an offset relative to the current position or some other buffer stored elsewhere in the file.

```rust
#[derive(BinRead, SsbhWrite)]
struct FileData {
    name: SsbhString,
    name_offset: RelPtr64<SsbhString>,
    values: SsbhArray<u32>    
}
```
Composing a combination of predefined SSBH types such as `SsbhString` with additional types implementing `SsbhWrite` and `BinRead` improves the amount of type information for the data and makes the usage of offsets less ambiguous. The code to read and write the data from the raw binary data is handled entirely by deriving `BinRead` and `SsbhWrite`.
