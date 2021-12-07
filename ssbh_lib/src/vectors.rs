use binread::BinRead;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use ssbh_write::SsbhWrite;

/// 3 contiguous floats for encoding XYZ or RGB data.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, PartialEq, SsbhWrite, Clone, Copy, Default)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub const ZERO: Self = Vector3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(x: f32, y: f32, z: f32) -> Vector3 {
        Vector3 { x, y, z }
    }

    /// Converts the vector elements to an array.
    /// # Examples
    /**
    ```rust
    # use ssbh_lib::Vector3;
    assert_eq!([1.0, 2.0, 3.0], Vector3::new(1.0, 2.0, 3.0).to_array());
    ```
     */
    pub fn to_array(&self) -> [f32; 3] {
        (*self).into()
    }

    /// Creates a [Vector4] from `self` and the given `w` component.
    /// # Examples
    /**
    ```rust
    # use ssbh_lib::{Vector3, Vector4};
    assert_eq!(Vector4::new(1.0, 2.0, 3.0, 4.0), Vector3::new(1.0, 2.0, 3.0).extend(4.0));
    ```
     */
    pub fn extend(&self, w: f32) -> Vector4 {
        Vector4 {
            x: self.x,
            y: self.y,
            z: self.z,
            w,
        }
    }

    /// Returns the component-wise min of the two vectors. See [f32::min].
    ///     
    /// # Examples
    /**
    ```rust
    # use ssbh_lib::Vector3;
    let a = Vector3::new(1.0, 2.0, 3.0);
    let b = Vector3::new(5.0, 6.0, 7.0);

    assert_eq!(a.min(b), a);
    ```
     */
    pub fn min(self, other: Vector3) -> Self {
        Self::new(
            f32::min(self.x, other.x),
            f32::min(self.y, other.y),
            f32::min(self.z, other.z),
        )
    }

    /// Returns the component-wise max of the two vectors. See [f32::max].
    ///
    /// # Examples
    /**
    ```rust
    # use ssbh_lib::Vector3;
    let a = Vector3::new(1.0, 2.0, 3.0);
    let b = Vector3::new(5.0, 6.0, 7.0);

    assert_eq!(a.max(b), b);
    ```
     */
    pub fn max(self, other: Vector3) -> Self {
        Self::new(
            f32::max(self.x, other.x),
            f32::max(self.y, other.y),
            f32::max(self.z, other.z),
        )
    }
}

impl From<(f32, f32, f32)> for Vector3 {
    fn from(v: (f32, f32, f32)) -> Self {
        Self {
            x: v.0,
            y: v.1,
            z: v.2,
        }
    }
}

impl From<Vector3> for (f32, f32, f32) {
    fn from(v: Vector3) -> Self {
        (v.x, v.y, v.z)
    }
}

impl From<[f32; 3]> for Vector3 {
    fn from(v: [f32; 3]) -> Self {
        Self {
            x: v[0],
            y: v[1],
            z: v[2],
        }
    }
}

impl From<Vector3> for [f32; 3] {
    fn from(v: Vector3) -> Self {
        [v.x, v.y, v.z]
    }
}

/// A row-major 3x3 matrix of contiguous floats.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, PartialEq, SsbhWrite, Clone, Copy, Default)]
pub struct Matrix3x3 {
    pub row1: Vector3,
    pub row2: Vector3,
    pub row3: Vector3,
}

impl Matrix3x3 {
    /// The identity transformation matrix.
    ///
    /**
    ```rust
    use ssbh_lib::{Vector3, Matrix3x3};

    let m = Matrix3x3::identity();
    assert_eq!(Vector3::new(1f32, 0f32, 0f32), m.row1);
    assert_eq!(Vector3::new(0f32, 1f32, 0f32), m.row2);
    assert_eq!(Vector3::new(0f32, 0f32, 1f32), m.row3);
    ```
    */
    pub fn identity() -> Matrix3x3 {
        Matrix3x3 {
            row1: Vector3::new(1f32, 0f32, 0f32),
            row2: Vector3::new(0f32, 1f32, 0f32),
            row3: Vector3::new(0f32, 0f32, 1f32),
        }
    }

    /// Converts the elements to a 2d array in row-major order.
    /**
    ```rust
    use ssbh_lib::{Vector3, Matrix3x3};

    let m = Matrix3x3 {
        row1: Vector3::new(1f32, 2f32, 3f32),
        row2: Vector3::new(4f32, 5f32, 6f32),
        row3: Vector3::new(7f32, 8f32, 9f32),
    };

    assert_eq!(
        [
            [1f32, 2f32, 3f32],
            [4f32, 5f32, 6f32],
            [7f32, 8f32, 9f32],
        ],
        m.to_rows_array(),
    );
    ```
    */
    pub fn to_rows_array(&self) -> [[f32; 3]; 3] {
        [
            [self.row1.x, self.row1.y, self.row1.z],
            [self.row2.x, self.row2.y, self.row2.z],
            [self.row3.x, self.row3.y, self.row3.z],
        ]
    }

    /// Creates the matrix from a 2d array in row-major order.
    /**
    ```rust
    # use ssbh_lib::Matrix3x3;
    let elements = [
        [1f32, 2f32, 3f32],
        [4f32, 5f32, 6f32],
        [7f32, 8f32, 9f32],
    ];
    let m = Matrix3x3::from_rows_array(&elements);
    assert_eq!(elements, m.to_rows_array());
    ```
    */
    pub fn from_rows_array(rows: &[[f32; 3]; 3]) -> Matrix3x3 {
        Matrix3x3 {
            row1: rows[0].into(),
            row2: rows[1].into(),
            row3: rows[2].into(),
        }
    }
}

/// 4 contiguous floats for encoding XYZW or RGBA data.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, PartialEq, SsbhWrite, Clone, Copy, Default)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vector4 {
    pub const ZERO: Self = Vector4 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };

    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Vector4 {
        Vector4 { x, y, z, w }
    }

    pub fn xyz(&self) -> Vector3 {
        Vector3::new(self.x, self.y, self.z)
    }

    /// Converts the vector elements to an array.
    /// # Examples
    /**
    ```rust
    # use ssbh_lib::Vector4;
    assert_eq!([1.0, 2.0, 3.0, 4.0], Vector4::new(1.0, 2.0, 3.0, 4.0).to_array());
    ```
     */
    pub fn to_array(&self) -> [f32; 4] {
        (*self).into()
    }

    /// Returns the component-wise min of the two vectors. See [f32::min].
    ///     
    /// # Examples
    /**
    ```rust
    # use ssbh_lib::Vector4;
    let a = Vector4::new(1.0, 2.0, 3.0, 4.0);
    let b = Vector4::new(5.0, 6.0, 7.0, 8.0);

    assert_eq!(a.min(b), a);
    ```
     */
    pub fn min(self, other: Vector4) -> Self {
        Self::new(
            f32::min(self.x, other.x),
            f32::min(self.y, other.y),
            f32::min(self.z, other.z),
            f32::min(self.w, other.w),
        )
    }

    /// Returns the component-wise max of the two vectors. See [f32::max].
    ///
    /// # Examples
    /**
    ```rust
    # use ssbh_lib::Vector4;
    let a = Vector4::new(1.0, 2.0, 3.0, 4.0);
    let b = Vector4::new(5.0, 6.0, 7.0, 8.0);

    assert_eq!(a.max(b), b);
    ```
     */
    pub fn max(self, other: Vector4) -> Self {
        Self::new(
            f32::max(self.x, other.x),
            f32::max(self.y, other.y),
            f32::max(self.z, other.z),
            f32::max(self.w, other.w),
        )
    }
}

impl From<(f32, f32, f32, f32)> for Vector4 {
    fn from(v: (f32, f32, f32, f32)) -> Self {
        Self {
            x: v.0,
            y: v.1,
            z: v.2,
            w: v.3,
        }
    }
}

impl From<Vector4> for (f32, f32, f32, f32) {
    fn from(v: Vector4) -> Self {
        (v.x, v.y, v.z, v.w)
    }
}

impl From<[f32; 4]> for Vector4 {
    fn from(v: [f32; 4]) -> Self {
        Self {
            x: v[0],
            y: v[1],
            z: v[2],
            w: v[3],
        }
    }
}

impl From<Vector4> for [f32; 4] {
    fn from(v: Vector4) -> Self {
        [v.x, v.y, v.z, v.w]
    }
}

/// 4 contiguous floats for encoding RGBA data.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, Clone, Copy, PartialEq, SsbhWrite)]
pub struct Color4f {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// A row-major 4x4 matrix of contiguous floats.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(BinRead, Debug, PartialEq, SsbhWrite)]
pub struct Matrix4x4 {
    pub row1: Vector4,
    pub row2: Vector4,
    pub row3: Vector4,
    pub row4: Vector4,
}

impl Matrix4x4 {
    /// The identity transformation matrix.
    ///
    /**
    ```rust
    use ssbh_lib::{Vector4, Matrix4x4};

    let m = Matrix4x4::identity();
    assert_eq!(Vector4::new(1f32, 0f32, 0f32, 0f32), m.row1);
    assert_eq!(Vector4::new(0f32, 1f32, 0f32, 0f32), m.row2);
    assert_eq!(Vector4::new(0f32, 0f32, 1f32, 0f32), m.row3);
    assert_eq!(Vector4::new(0f32, 0f32, 0f32, 1f32), m.row4);
    ```
    */
    pub fn identity() -> Matrix4x4 {
        Matrix4x4 {
            row1: Vector4::new(1f32, 0f32, 0f32, 0f32),
            row2: Vector4::new(0f32, 1f32, 0f32, 0f32),
            row3: Vector4::new(0f32, 0f32, 1f32, 0f32),
            row4: Vector4::new(0f32, 0f32, 0f32, 1f32),
        }
    }

    /// Converts the elements to a 2d array in row-major order.
    /**
    ```rust
    use ssbh_lib::{Vector4, Matrix4x4};

    let m = Matrix4x4 {
        row1: Vector4::new(1f32, 2f32, 3f32, 4f32),
        row2: Vector4::new(5f32, 6f32, 7f32, 8f32),
        row3: Vector4::new(9f32, 10f32, 11f32, 12f32),
        row4: Vector4::new(13f32, 14f32, 15f32, 16f32),
    };

    assert_eq!(
        [
            [1f32, 2f32, 3f32, 4f32],
            [5f32, 6f32, 7f32, 8f32],
            [9f32, 10f32, 11f32, 12f32],
            [13f32, 14f32, 15f32, 16f32],
        ],
        m.to_rows_array(),
    );
    ```
    */
    pub fn to_rows_array(&self) -> [[f32; 4]; 4] {
        [
            [self.row1.x, self.row1.y, self.row1.z, self.row1.w],
            [self.row2.x, self.row2.y, self.row2.z, self.row2.w],
            [self.row3.x, self.row3.y, self.row3.z, self.row3.w],
            [self.row4.x, self.row4.y, self.row4.z, self.row4.w],
        ]
    }

    /// Creates the matrix from a 2d array in row-major order.
    /**
    ```rust
    # use ssbh_lib::Matrix4x4;
    let elements = [
        [1f32, 2f32, 3f32, 4f32],
        [5f32, 6f32, 7f32, 8f32],
        [9f32, 10f32, 11f32, 12f32],
        [13f32, 14f32, 15f32, 16f32],
    ];
    let m = Matrix4x4::from_rows_array(&elements);
    assert_eq!(elements, m.to_rows_array());
    ```
    */
    pub fn from_rows_array(rows: &[[f32; 4]; 4]) -> Matrix4x4 {
        Matrix4x4 {
            row1: rows[0].into(),
            row2: rows[1].into(),
            row3: rows[2].into(),
            row4: rows[3].into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use binread::BinReaderExt;
    use std::io::Cursor;

    use hexlit::hex;

    use super::*;

    #[test]
    fn vector3_conversions() {
        assert_eq!((1.0, 2.0, 3.0), Vector3::new(1.0, 2.0, 3.0).into());
        assert_eq!(
            [1.0, 2.0, 3.0],
            <[f32; 3]>::from(Vector3::new(1.0, 2.0, 3.0))
        );
        assert_eq!(Vector3::new(1.0, 2.0, 3.0), (1.0, 2.0, 3.0).into());
        assert_eq!(Vector3::new(1.0, 2.0, 3.0), [1.0, 2.0, 3.0].into());
    }

    #[test]
    fn vector4_conversions() {
        assert_eq!(
            (1.0, 2.0, 3.0, 4.0),
            Vector4::new(1.0, 2.0, 3.0, 4.0).into()
        );
        assert_eq!(
            [1.0, 2.0, 3.0, 4.0],
            <[f32; 4]>::from(Vector4::new(1.0, 2.0, 3.0, 4.0))
        );
        assert_eq!(
            Vector4::new(1.0, 2.0, 3.0, 4.0),
            (1.0, 2.0, 3.0, 4.0).into()
        );
        assert_eq!(
            Vector4::new(1.0, 2.0, 3.0, 4.0),
            [1.0, 2.0, 3.0, 4.0].into()
        );
    }

    #[test]
    fn read_vector3() {
        let mut reader = Cursor::new(hex!("0000803F 000000C0 0000003F"));
        let value = reader.read_le::<Vector3>().unwrap();
        assert_eq!(1.0f32, value.x);
        assert_eq!(-2.0f32, value.y);
        assert_eq!(0.5f32, value.z);
    }

    #[test]
    fn read_vector4() {
        let mut reader = Cursor::new(hex!("0000803F 000000C0 0000003F 0000803F"));
        let value = reader.read_le::<Vector4>().unwrap();
        assert_eq!(1.0f32, value.x);
        assert_eq!(-2.0f32, value.y);
        assert_eq!(0.5f32, value.z);
        assert_eq!(1.0f32, value.w);
    }

    #[test]
    fn read_color4f() {
        let mut reader = Cursor::new(hex!("0000803E 0000003F 0000003E 0000803F"));
        let value = reader.read_le::<Vector4>().unwrap();
        assert_eq!(0.25f32, value.x);
        assert_eq!(0.5f32, value.y);
        assert_eq!(0.125f32, value.z);
        assert_eq!(1.0f32, value.w);
    }

    #[test]
    fn read_matrix4x4_identity() {
        let mut reader = Cursor::new(hex!(
            "0000803F 00000000 00000000 00000000 
             00000000 0000803F 00000000 00000000 
             00000000 00000000 0000803F 00000000 
             00000000 00000000 00000000 0000803F"
        ));
        let value = reader.read_le::<Matrix4x4>().unwrap();
        assert_eq!(Vector4::new(1f32, 0f32, 0f32, 0f32), value.row1);
        assert_eq!(Vector4::new(0f32, 1f32, 0f32, 0f32), value.row2);
        assert_eq!(Vector4::new(0f32, 0f32, 1f32, 0f32), value.row3);
        assert_eq!(Vector4::new(0f32, 0f32, 0f32, 1f32), value.row4);
    }

    #[test]
    fn read_matrix3x3_identity() {
        let mut reader = Cursor::new(hex!(
            "0000803F 00000000 00000000 
             00000000 0000803F 00000000 
             00000000 00000000 0000803F"
        ));
        let value = reader.read_le::<Matrix3x3>().unwrap();
        assert_eq!(Vector3::new(1f32, 0f32, 0f32), value.row1);
        assert_eq!(Vector3::new(0f32, 1f32, 0f32), value.row2);
        assert_eq!(Vector3::new(0f32, 0f32, 1f32), value.row3);
    }
}
