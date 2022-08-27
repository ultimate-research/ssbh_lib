# Architecture
## `ssbh_lib/`
### Overview
ssbh_lib is the lowest level API and implements binary format reading and writing. 

### Formats
Each format consists of types that contain the minimal amount of attributes that can be used to fully represent the binary data stored in the file. This ensures reading and writing an SSBH file produces a binary identical output as much as possible. 

Invalid state for the binary format should be unrepresentable in Rust. If the code compiles, writing the file should always result in a valid binary file.

Each format is entirely self contained in the format file like `src/formats/matl.rs` for `.numbatb` files. 

### Fuzz Testing
Fuzz tests are contained in the `ssbh_lib/fuzz` directory and can be run using `cargo fuzz`. The different fuzz targets test that reading, writing, and then reading each of the formats results in binary identical output and identical Rust structs. This is a simple way to check for data loss on write, unrepresentable data, behavior on malformed binary inputs, etc. The fuzz tests for array types check for robustness to malformed data like an invalid length.

## `ssbh_data/`
### Overview
ssbh_data is a higher level API intended for use in application code. The API for ssbh_data is less verbose, less likely to experience breaking changes, and abstracts away the details of managing format version differences and byte buffer layouts. 

Producing binary identical output is not the primary goal of ssbh_data. Certain complex operations such as lossy animation compression are handled implicitly, so output files may not be binary identical with their respective inputs even without modifications. Certain non essential fields are not preserved like the original file name used to generate an Anim file.

### Formats
Each format consists of types that contain the minimal set of attributes needed to reproduce the underlying ssbh_lib types. This ensures that mutating object attributes does not put the data in an inconsistent state. An example of inconsistent state would be removing a mesh attribute in ssbh_lib without altering the vertex buffer. The avoidance of readonly fields means users can modify fields in JSON form for ssbh_data_json or in Python with [ssbh_data_py](https://github.com/ScanMountGoat/ssbh_data_py) and expect consistent results.

This approach was inspired by relational database design where the process is known as [normalization](https://en.wikipedia.org/wiki/Database_normalization). Database normalization isn't directly applicable to this library since the data is hierarchical rather than relational.

Anim track value compression and decompression logic is contained in `src/anim_data/compression.rs`. The actual creation of the animation buffer and associated tests are contained in `src/anim_data/buffers.rs`.

Mesh vertex attribute reading and writing logic is contained in `src/mesh_data/vector_data.rs`. ssbh_data uses a unified format for mesh attributes for all versions of the Mesh format. These conversions are contained in `src/mesh_data/mesh_attributes.rs`.

### Fuzz Testing
Fuzz tests are contained in the `ssbh_data/fuzz` directory and can be run using `cargo fuzz`. The different fuzz targets test the conversions between each of the ssbh_data format types and its corresponding ssbh_lib type like `MatlData` and `Matl`.