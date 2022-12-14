use binrw::io::SeekFrom;

use crate::mesh::BoundingSphere;
use crate::{CString, Ptr64, Vector3};
use binrw::{binread, BinRead};
use modular_bitfield::prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use ssbh_write::SsbhWrite;

/// Extended mesh data and bounding spheres for .numshexb files.
#[binread]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq)]
pub struct MeshEx {
    #[br(temp)]
    file_length: u64,

    #[br(temp)]
    entry_count: u32,

    #[br(temp)]
    mesh_object_group_count: u32,

    pub all_data: Ptr64<AllData>,

    #[br(count = mesh_object_group_count)]
    pub mesh_object_groups: Ptr64<Vec<MeshObjectGroup>>,

    #[br(count = entry_count)]
    pub entries: Ptr64<Vec<MeshEntry>>,

    // TODO: Find a way to set the alignment without creating a new type.
    #[br(args(entry_count as usize))]
    pub entry_flags: Ptr64<EntryFlags>,

    pub unk1: u32,
}

// TODO: How does MeshEx handle empty strings?
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, Clone, PartialEq)]
#[ssbhwrite(alignment = 16)]
pub struct MeshEntry {
    /// The index of the corresponding [MeshObject](crate::formats::mesh::MeshObject) when grouped by name.
    /// If multiple [MeshObject](crate::formats::mesh::MeshObject) share the same name,
    /// the [mesh_object_group_index](#structfield.mesh_object_group_index) will be the same.
    pub mesh_object_group_index: u32,
    pub unk1: Vector3,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, Clone, PartialEq)]
#[ssbhwrite(alignment = 16)]
pub struct AllData {
    pub bounding_sphere: BoundingSphere,
    pub name: Ptr64<CString<16>>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, Clone, PartialEq)]
#[ssbhwrite(alignment = 16)]
pub struct MeshObjectGroup {
    /// The combined bounding sphere information for mesh objects with the same name.
    pub bounding_sphere: BoundingSphere,
    /// The name of the [MeshObject](crate::formats::mesh::MeshObject) including the tag such as "Mario_FaceN_VIS_O_OBJShape".
    pub mesh_object_full_name: Ptr64<CString<4>>,
    /// The name of the [MeshObject](crate::formats::mesh::MeshObject) such as "Mario_FaceN".
    pub mesh_object_name: Ptr64<CString<4>>,
}

#[bitfield(bits = 16)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Clone, Copy, PartialEq, Eq)]
#[br(map = Self::from_bytes)]
pub struct EntryFlag {
    pub draw_model: bool,
    pub cast_shadow: bool,
    #[skip]
    __: bool,
    // TODO: Disables reflection of stage model in Fountain of Dreams's water.
    pub unk3: bool,
    // TODO: Only draws stage model in Fountain of Dreams's water reflection.
    pub unk4: bool,
    // TODO: Used for "light_neck_VIS_O_OBJShape" and "light_neck_lowShape" with subindices 1 in fighter/jack/model/doyle/c00/
    pub unk5: bool,
    #[skip]
    __: B10,
}

ssbh_write::ssbh_write_modular_bitfield_impl!(EntryFlag, 2);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, SsbhWrite, Clone, PartialEq)]
#[ssbhwrite(alignment = 16)]
#[br(import(count: usize))]
pub struct EntryFlags(#[br(count = count)] pub Vec<EntryFlag>);

impl SsbhWrite for MeshEx {
    fn ssbh_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        data_ptr: &mut u64,
    ) -> std::io::Result<()> {
        // Check for invalid lengths before writing any data.
        let entry_count = self
            .entries
            .as_ref()
            .map(|e| e.len() as u32)
            .unwrap_or(0u32);
        let entry_flag_count = self
            .entry_flags
            .as_ref()
            .map(|e| e.0.len() as u32)
            .unwrap_or(0u32);

        if entry_count != entry_flag_count {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Inconsistent entry count: {entry_count} != {entry_flag_count}"),
            ));
        }

        // Ensure the next pointer won't point inside this struct.
        let current_pos = writer.stream_position()?;
        if *data_ptr < current_pos + self.size_in_bytes() {
            *data_ptr = current_pos + self.size_in_bytes();
        }

        // Write all the fields.
        // Use a placeholder for file length.
        (0u64).ssbh_write(writer, data_ptr)?;
        entry_count.ssbh_write(writer, data_ptr)?;

        self.mesh_object_groups
            .as_ref()
            .map(|g| g.len() as u32)
            .unwrap_or(0u32)
            .ssbh_write(writer, data_ptr)?;

        self.all_data.ssbh_write(writer, data_ptr)?;
        self.mesh_object_groups.ssbh_write(writer, data_ptr)?;
        self.entries.ssbh_write(writer, data_ptr)?;
        self.entry_flags.ssbh_write(writer, data_ptr)?;
        self.unk1.ssbh_write(writer, data_ptr)?;

        // Meshex files are aligned to 16 bytes.
        let round_up = |value, n| ((value + n - 1) / n) * n;
        let size = writer.seek(SeekFrom::End(0))?;
        let new_size = round_up(size, 16);
        writer.write_all(&vec![0u8; (new_size - size) as usize])?;

        // Write the file length.
        writer.rewind()?;
        new_size.ssbh_write(writer, data_ptr)?;
        Ok(())
    }

    fn size_in_bytes(&self) -> u64 {
        // header + padding
        64
    }
}
