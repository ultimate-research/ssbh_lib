use crate::{InlineString, Ptr64, Vector3, Vector4};
use binread::BinRead;
use serde::Serialize;

#[derive(Serialize, BinRead, Debug)]
pub struct MeshEntry {
    mesh_index: i32,
    unk1: Vector3,
}

#[derive(Serialize, BinRead, Debug)]
pub struct AllData {
    bounding_sphere: Vector4,
    name: Ptr64<InlineString>,
}

#[derive(Serialize, BinRead, Debug)]
pub struct MeshData {
    bounding_sphere: Vector4,
    name: Ptr64<InlineString>,
    actual_name: Ptr64<InlineString>,
}

/// Extended mesh data and bounding spheres for .numshexb files.
#[derive(Serialize, BinRead, Debug)]
pub struct MeshEx {
    file_length: u64,
    entry_count: u32,
    mesh_data_count: u32,
    all_data: Ptr64<AllData>,

    #[br(count = mesh_data_count)]
    mesh_data: Ptr64<Vec<MeshData>>,

    #[br(count = entry_count)]
    entries: Ptr64<Vec<MeshEntry>>,

    #[br(count = entry_count)]
    entry_flags: Ptr64<Vec<u16>>,
}
