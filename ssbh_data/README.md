# ssbh_data [![Latest Version](https://img.shields.io/crates/v/ssbh_data.svg)](https://crates.io/crates/ssbh_data) [![docs.rs](https://docs.rs/ssbh_data/badge.svg)](https://docs.rs/ssbh_data)  
A higher level data access layer for some SSBH formats. ssbh_data provides a more intuitive and minimal API where possible. SSBH types like `SsbhArray` and `SsbhString8` are replaced with their standard Rust equivalents of `Vec` and `String`. The decoding and encoding of binary buffers is handled automatically for formats like mesh and anim. Python bindings are available with [ssbh_data_py](https://github.com/ScanMountGoat/ssbh_data_py). 

## Supported Formats
| Format | Supported Versions (major.minor) | Read | Save |
| --- | --- | --- | --- |
| Modl (`.numdlb`, `.nusrcmdlb`) | 1.7 | :heavy_check_mark: | :heavy_check_mark: |
| Mesh (`.numshb`) | 1.10 | :heavy_check_mark: | :heavy_check_mark: |
| Skel (`.nusktb`) | 1.0 | :heavy_check_mark: | :heavy_check_mark: |
| Anim (`.nuanmb`) | 2.0 | :heavy_check_mark: | :x: |