use crate::{InlineString, Ptr64, Vector3, Vector4};
use binread::BinRead;

#[cfg(feature = "derive_serde")]
use serde::Serialize;

#[cfg_attr(feature = "derive_serde", derive(Serialize))]
#[derive(BinRead, Debug)]
pub struct MeshEntry {
    pub mesh_index: i32,
    pub unk1: Vector3,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize))]
#[derive(BinRead, Debug)]
pub struct AllData {
    pub bounding_sphere: Vector4,
    pub name: Ptr64<InlineString>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize))]
#[derive(BinRead, Debug)]
pub struct MeshData {
    pub bounding_sphere: Vector4,
    pub name: Ptr64<InlineString>,
    pub actual_name: Ptr64<InlineString>,
}

/// Extended mesh data and bounding spheres for .numshexb files.
#[cfg_attr(feature = "derive_serde", derive(Serialize))]
#[derive(BinRead, Debug)]
pub struct MeshEx {
    pub file_length: u64,
    pub entry_count: u32,
    pub mesh_data_count: u32,
    pub all_data: Ptr64<AllData>,

    #[br(count = mesh_data_count)]
    pub mesh_data: Ptr64<Vec<MeshData>>,

    #[br(count = entry_count)]
    pub entries: Ptr64<Vec<MeshEntry>>,

    #[br(count = entry_count)]
    pub entry_flags: Ptr64<Vec<u16>>,
}
