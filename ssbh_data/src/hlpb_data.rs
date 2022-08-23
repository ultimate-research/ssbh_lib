//! Types for working with [Hlpb] data in .nuhlpb files.
use std::iter::repeat;

use ssbh_lib::{formats::hlpb::*, Vector3, Vector4};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The data associated with a [Hlpb] file.
/// The supported version is 1.0.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct HlpbData {
    pub major_version: u16,
    pub minor_version: u16,
    pub aim_constraints: Vec<AimConstraintData>,
    pub orient_constraints: Vec<OrientConstraintData>,
}

// TODO: Simplify these fields?
// TODO: Use clearer field names.
/// Data associated with an [AimConstraint].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct AimConstraintData {
    pub name: String,
    pub aim_bone_name1: String,
    pub aim_bone_name2: String,
    pub aim_type1: String,
    pub aim_type2: String,
    pub target_bone_name1: String,
    pub target_bone_name2: String,
    pub unk1: u32,
    pub unk2: u32,
    pub aim: Vector3,
    pub up: Vector3,
    pub quat1: Vector4,
    pub quat2: Vector4,
}

/// Data associated with an [OrientConstraint].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct OrientConstraintData {
    pub name: String,
    pub parent_bone_name1: String,
    pub parent_bone_name2: String,
    pub source_bone_name: String,
    pub target_bone_name: String,
    pub unk_type: u32,
    pub constraint_axes: Vector3,
    pub quat1: Vector4,
    pub quat2: Vector4,
    pub range_min: Vector3,
    pub range_max: Vector3,
}

// Define two way conversions between types.
impl From<Hlpb> for HlpbData {
    fn from(h: Hlpb) -> Self {
        Self::from(&h)
    }
}

impl From<&Hlpb> for HlpbData {
    fn from(h: &Hlpb) -> Self {
        match h {
            Hlpb::V11 {
                aim_constraints,
                orient_constraints,
                ..
            } => Self {
                major_version: 1,
                minor_version: 0,
                aim_constraints: aim_constraints.elements.iter().map(Into::into).collect(),
                orient_constraints: orient_constraints.elements.iter().map(Into::into).collect(),
            },
        }
    }
}

impl From<HlpbData> for Hlpb {
    fn from(data: HlpbData) -> Self {
        Self::from(&data)
    }
}

impl From<&HlpbData> for Hlpb {
    fn from(data: &HlpbData) -> Self {
        Self::V11 {
            aim_constraints: data.aim_constraints.iter().map(Into::into).collect(),
            orient_constraints: data.orient_constraints.iter().map(Into::into).collect(),
            constraint_indices: (0..data.aim_constraints.len() as u32)
                .chain(0..data.orient_constraints.len() as u32)
                .collect(),
            constraint_types: repeat(ConstraintType::Aim)
                .take(data.aim_constraints.len())
                .chain(repeat(ConstraintType::Orient).take(data.orient_constraints.len()))
                .collect(),
        }
    }
}

impl From<AimConstraint> for AimConstraintData {
    fn from(a: AimConstraint) -> Self {
        Self::from(&a)
    }
}

impl From<&AimConstraint> for AimConstraintData {
    fn from(a: &AimConstraint) -> Self {
        Self {
            name: a.name.to_string_lossy(),
            aim_bone_name1: a.aim_bone_name1.to_string_lossy(),
            aim_bone_name2: a.aim_bone_name2.to_string_lossy(),
            aim_type1: a.aim_type1.to_string_lossy(),
            aim_type2: a.aim_type2.to_string_lossy(),
            target_bone_name1: a.target_bone_name1.to_string_lossy(),
            target_bone_name2: a.target_bone_name2.to_string_lossy(),
            unk1: a.unk1,
            unk2: a.unk2,
            aim: a.aim,
            up: a.up,
            quat1: a.quat1,
            quat2: a.quat2,
        }
    }
}

impl From<AimConstraintData> for AimConstraint {
    fn from(a: AimConstraintData) -> Self {
        Self::from(&a)
    }
}

impl From<&AimConstraintData> for AimConstraint {
    fn from(a: &AimConstraintData) -> Self {
        Self {
            name: a.name.as_str().into(),
            aim_bone_name1: a.aim_bone_name1.as_str().into(),
            aim_bone_name2: a.aim_bone_name2.as_str().into(),
            aim_type1: a.aim_type1.as_str().into(),
            aim_type2: a.aim_type2.as_str().into(),
            target_bone_name1: a.target_bone_name1.as_str().into(),
            target_bone_name2: a.target_bone_name2.as_str().into(),
            unk1: a.unk1,
            unk2: a.unk2,
            aim: a.aim,
            up: a.up,
            quat1: a.quat1,
            quat2: a.quat2,
            // TODO: Are these fields always 0?
            unk17: 0.0,
            unk18: 0.0,
            unk19: 0.0,
            unk20: 0.0,
            unk21: 0.0,
            unk22: 0.0,
        }
    }
}

impl From<OrientConstraint> for OrientConstraintData {
    fn from(o: OrientConstraint) -> Self {
        Self::from(&o)
    }
}

impl From<&OrientConstraint> for OrientConstraintData {
    fn from(o: &OrientConstraint) -> Self {
        Self {
            name: o.name.to_string_lossy(),
            parent_bone_name1: o.parent_bone_name1.to_string_lossy(),
            parent_bone_name2: o.parent_bone_name2.to_string_lossy(),
            source_bone_name: o.source_bone_name.to_string_lossy(),
            target_bone_name: o.target_bone_name.to_string_lossy(),
            unk_type: o.unk_type,
            constraint_axes: o.constraint_axes,
            quat1: o.quat1,
            quat2: o.quat2,
            range_min: o.range_min,
            range_max: o.range_max,
        }
    }
}

impl From<OrientConstraintData> for OrientConstraint {
    fn from(o: OrientConstraintData) -> Self {
        Self::from(&o)
    }
}

impl From<&OrientConstraintData> for OrientConstraint {
    fn from(o: &OrientConstraintData) -> Self {
        Self {
            name: o.name.as_str().into(),
            parent_bone_name1: o.parent_bone_name1.as_str().into(),
            parent_bone_name2: o.parent_bone_name2.as_str().into(),
            source_bone_name: o.source_bone_name.as_str().into(),
            target_bone_name: o.target_bone_name.as_str().into(),
            unk_type: o.unk_type,
            constraint_axes: o.constraint_axes,
            quat1: o.quat1,
            quat2: o.quat2,
            range_min: o.range_min,
            range_max: o.range_max,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_hlpb_hlpb_data() {
        // Test both conversion directions.
        let ssbh = Hlpb::V11 {
            aim_constraints: vec![AimConstraint {
                name: "aim1".into(),
                aim_bone_name1: "root".into(),
                aim_bone_name2: "root".into(),
                aim_type1: "DEFAULT".into(),
                aim_type2: "DEFAULT".into(),
                target_bone_name1: "a".into(),
                target_bone_name2: "a".into(),
                unk1: 0,
                unk2: 1,
                aim: Vector3::new(1.0, 0.0, 0.0),
                up: Vector3::new(0.0, 1.0, 0.0),
                quat1: Vector4::new(0.0, 0.0, 0.0, 1.0),
                quat2: Vector4::new(0.0, 0.0, 0.0, 1.0),
                unk17: 0.0,
                unk18: 0.0,
                unk19: 0.0,
                unk20: 0.0,
                unk21: 0.0,
                unk22: 0.0,
            }]
            .into(),
            orient_constraints: vec![
                OrientConstraint {
                    name: "orient1".into(),
                    parent_bone_name1: "ArmL".into(),
                    parent_bone_name2: "ArmL".into(),
                    source_bone_name: "HandL".into(),
                    target_bone_name: "H_WristL".into(),
                    unk_type: 2,
                    constraint_axes: Vector3::new(0.5, 0.5, 0.5),
                    quat1: Vector4::new(0.0, 0.0, 0.0, 1.0),
                    quat2: Vector4::new(0.0, 0.0, 0.0, 1.0),
                    range_min: Vector3::new(-180.0, -180.0, -180.0),
                    range_max: Vector3::new(180.0, 180.0, 180.0),
                },
                OrientConstraint {
                    name: "orient2".into(),
                    parent_bone_name1: "ArmR".into(),
                    parent_bone_name2: "ArmR".into(),
                    source_bone_name: "HandR".into(),
                    target_bone_name: "H_WristR".into(),
                    unk_type: 2,
                    constraint_axes: Vector3::new(0.5, 0.5, 0.5),
                    quat1: Vector4::new(0.0, 0.0, 0.0, 1.0),
                    quat2: Vector4::new(0.0, 0.0, 0.0, 1.0),
                    range_min: Vector3::new(-180.0, -180.0, -180.0),
                    range_max: Vector3::new(180.0, 180.0, 180.0),
                },
            ]
            .into(),
            constraint_indices: vec![0, 0, 1].into(),
            constraint_types: vec![
                ConstraintType::Aim,
                ConstraintType::Orient,
                ConstraintType::Orient,
            ]
            .into(),
        };

        let data = HlpbData {
            major_version: 1,
            minor_version: 0,
            aim_constraints: vec![AimConstraintData {
                name: "aim1".to_string(),
                aim_bone_name1: "root".to_string(),
                aim_bone_name2: "root".to_string(),
                aim_type1: "DEFAULT".to_string(),
                aim_type2: "DEFAULT".to_string(),
                target_bone_name1: "a".to_string(),
                target_bone_name2: "a".to_string(),
                unk1: 0,
                unk2: 1,
                aim: Vector3::new(1.0, 0.0, 0.0),
                up: Vector3::new(0.0, 1.0, 0.0),
                quat1: Vector4::new(0.0, 0.0, 0.0, 1.0),
                quat2: Vector4::new(0.0, 0.0, 0.0, 1.0),
            }],
            orient_constraints: vec![
                OrientConstraintData {
                    name: "orient1".to_string(),
                    parent_bone_name1: "ArmL".to_string(),
                    parent_bone_name2: "ArmL".to_string(),
                    source_bone_name: "HandL".to_string(),
                    target_bone_name: "H_WristL".to_string(),
                    unk_type: 2,
                    constraint_axes: Vector3::new(0.5, 0.5, 0.5),
                    quat1: Vector4::new(0.0, 0.0, 0.0, 1.0),
                    quat2: Vector4::new(0.0, 0.0, 0.0, 1.0),
                    range_min: Vector3::new(-180.0, -180.0, -180.0),
                    range_max: Vector3::new(180.0, 180.0, 180.0),
                },
                OrientConstraintData {
                    name: "orient2".to_string(),
                    parent_bone_name1: "ArmR".to_string(),
                    parent_bone_name2: "ArmR".to_string(),
                    source_bone_name: "HandR".to_string(),
                    target_bone_name: "H_WristR".to_string(),
                    unk_type: 2,
                    constraint_axes: Vector3::new(0.5, 0.5, 0.5),
                    quat1: Vector4::new(0.0, 0.0, 0.0, 1.0),
                    quat2: Vector4::new(0.0, 0.0, 0.0, 1.0),
                    range_min: Vector3::new(-180.0, -180.0, -180.0),
                    range_max: Vector3::new(180.0, 180.0, 180.0),
                },
            ],
        };

        assert_eq!(data, HlpbData::from(&ssbh));
        assert_eq!(ssbh, Hlpb::from(&data));
    }
}
