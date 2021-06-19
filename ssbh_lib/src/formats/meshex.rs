use crate::{CString, InlineString, Ptr64, Vector3, Vector4};
use binread::BinRead;

#[cfg(feature = "derive_serde")]
use serde::{Serialize, Deserialize};

use ssbh_write_derive::SsbhWrite;

// 16 byte alignment for pointers.
// Header is 64 bytes?

// TODO: Derive ssbhwrite even though this isn't an SSBH type?
// TODO: Test the write function for Ptr64
// TODO: How does MeshEx handle empty strings?
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct MeshEntry {
    /// The index of the corresponding [MeshObject](crate::formats::mesh::MeshObject) when grouped by name.
    /// If multiple [MeshObject](crate::formats::mesh::MeshObject) share the same name,
    /// the [mesh_object_index](#structfield.mesh_object_index) will be the same.
    pub mesh_object_index: u32,
    pub unk1: Vector3,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct AllData {
    pub bounding_sphere: Vector4,
    pub name: Ptr64<CString<16>>,
}

#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
pub struct MeshObjectData {
    // TODO: The combined bounding information for mesh objects with the same name?
    pub bounding_sphere: Vector4,
    /// The name of the [MeshObject](crate::formats::mesh::MeshObject) including the tag such as "Mario_FaceN_VIS_O_OBJShape".
    pub mesh_object_full_name: Ptr64<InlineString>,
    /// The name of the [MeshObject](crate::formats::mesh::MeshObject) such as "Mario_FaceN".
    pub mesh_object_name: Ptr64<InlineString>,
}

/// Extended mesh data and bounding spheres for .numshexb files.
#[cfg_attr(feature = "derive_serde", derive(Serialize, Deserialize))]
#[derive(BinRead, Debug, SsbhWrite)]
#[ssbhwrite(pad_after = 16)]
pub struct MeshEx {
    pub file_length: u64,
    pub entry_count: u32,
    pub mesh_object_data_count: u32,
    pub all_data: Ptr64<AllData>,

    #[br(count = mesh_object_data_count)]
    pub mesh_object_data: Ptr64<Vec<MeshObjectData>>,

    #[br(count = entry_count)]
    pub entries: Ptr64<Vec<MeshEntry>>,

    #[br(count = entry_count)]
    pub entry_flags: Ptr64<Vec<u16>>,
}