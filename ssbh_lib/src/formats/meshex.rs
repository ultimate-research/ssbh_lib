use crate::{CString, Ptr64, Vector3, Vector4};
use binread::BinRead;
use modular_bitfield::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use ssbh_write::SsbhWrite;
// TODO: How does MeshEx handle empty strings?
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[ssbhwrite(alignment = 16)]
pub struct MeshEntry {
    /// The index of the corresponding [MeshObject](crate::formats::mesh::MeshObject) when grouped by name.
    /// If multiple [MeshObject](crate::formats::mesh::MeshObject) share the same name,
    /// the [mesh_object_index](#structfield.mesh_object_index) will be the same.
    pub mesh_object_index: u32,
    pub unk1: Vector3,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[ssbhwrite(alignment = 16)]
pub struct AllData {
    pub bounding_sphere: Vector4,
    pub name: Ptr64<CString<16>>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[ssbhwrite(alignment = 16)]
pub struct MeshObjectGroup {
    // TODO: The combined bounding information for mesh objects with the same name?
    pub bounding_sphere: Vector4,
    /// The name of the [MeshObject](crate::formats::mesh::MeshObject) including the tag such as "Mario_FaceN_VIS_O_OBJShape".
    pub mesh_object_full_name: Ptr64<CString<4>>,
    /// The name of the [MeshObject](crate::formats::mesh::MeshObject) such as "Mario_FaceN".
    pub mesh_object_name: Ptr64<CString<4>>,
}

#[bitfield(bits = 16)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Clone, Copy)]
#[br(map = Self::from_bytes)]
pub struct EntryFlag {
    draw_model: bool,
    cast_shadow: bool,
    #[skip]
    __: bool,
    unk3: bool,
    unk4: bool,
    unk5: bool,
    #[skip]
    __: B10,
}

ssbh_write::ssbh_write_modular_bitfield_impl!(EntryFlag, 2);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[ssbhwrite(alignment = 16)]
pub struct EntryFlags(pub Vec<EntryFlag>);

/// Extended mesh data and bounding spheres for .numshexb files.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, SsbhWrite)]
#[ssbhwrite(align_after = 16)]
#[ssbhwrite(pad_after = 12)]
pub struct MeshEx {
    pub file_length: u64,
    pub entry_count: u32,
    pub mesh_object_group_count: u32,

    pub all_data: Ptr64<AllData>,

    #[br(count = mesh_object_group_count)]
    pub mesh_object_group: Ptr64<Vec<MeshObjectGroup>>,

    #[br(count = entry_count)]
    pub entries: Ptr64<Vec<MeshEntry>>,

    // TODO: Find a way to set the alignment without creating a new type.
    #[br(count = entry_count)]
    pub entry_flags: Ptr64<EntryFlags>,

    pub unk1: u32,
}
