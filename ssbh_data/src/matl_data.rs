pub use ssbh_lib::formats::matl::{
    BlendFactor, CullMode, FillMode, FilteringType, MagFilter, MaxAnisotropy, MinFilter, ParamId,
    WrapMode,
};
pub use ssbh_lib::{Color4f, Vector4};
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::SsbhData;

/// Errors while creating a [Matl] from [MatlData].
#[derive(Error, Debug)]

pub enum MatlError {
    /// Creating a [Matl] file for the given version is not supported.
    #[error(
        "Creating a version {}.{} matl is not supported.",
        major_version,
        minor_version
    )]
    UnsupportedVersion {
        major_version: u16,
        minor_version: u16,
    },
}

/// The data associated with a [Matl] file.
/// The supported version is 1.6.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct MatlData {
    pub major_version: u16,
    pub minor_version: u16,
    pub entries: Vec<MatlEntryData>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct MatlEntryData {
    pub material_label: String,
    pub shader_label: String,
    pub vectors: Vec<ParamData<Vector4>>,
    pub floats: Vec<ParamData<f32>>,
    pub booleans: Vec<ParamData<bool>>,
    pub textures: Vec<ParamData<String>>,
    pub samplers: Vec<ParamData<SamplerData>>,
    pub blend_states: Vec<ParamData<BlendStateData>>,
    pub rasterizer_states: Vec<ParamData<RasterizerStateData>>,
    // TODO: UV Transform?
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct ParamData<T: std::fmt::Debug> {
    param_id: ParamId,
    data: T,
}

// TODO: Derive default for these types to make them easier to use.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct SamplerData {
    pub wraps: WrapMode,
    pub wrapt: WrapMode,
    pub wrapr: WrapMode,
    pub min_filter: MinFilter,
    pub mag_filter: MagFilter,
    /// The color when sampling texture coordinates outside the 0 to 1 range.
    /// This only applies to [WrapMode::ClampToBorder].
    pub border_color: Color4f,
    pub lod_bias: f32,
    /// The amount of anisotropic filtering to used.
    /// A value of [None] disables anisotropic filtering.
    pub max_anisotropy: Option<MaxAnisotropy>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct BlendStateData {
    pub source_color: BlendFactor,
    pub destination_color: BlendFactor,
    pub alpha_sample_to_coverage: bool,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct RasterizerStateData {
    pub fill_mode: FillMode,
    pub cull_mode: CullMode,
    pub depth_bias: f32,
}

impl SsbhData for MatlData {
    type WriteError = MatlError;

    fn from_file<P: AsRef<std::path::Path>>(_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        todo!()
    }

    fn read<R: std::io::Read + std::io::Seek>(
        _reader: &mut R,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        todo!()
    }

    fn write<W: std::io::Write + std::io::Seek>(
        &self,
        _writer: &mut W,
    ) -> Result<(), Self::WriteError> {
        todo!()
    }

    fn write_to_file<P: AsRef<std::path::Path>>(&self, _path: P) -> Result<(), Self::WriteError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {

    // TODO: Test both directions for conversions.
    // TODO: Test the supported versions
    // TODO: Test the order for saved parameters.
}
