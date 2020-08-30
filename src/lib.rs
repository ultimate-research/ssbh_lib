pub mod utils {
    use binread::{
        io::{Read, Seek, SeekFrom},
        BinRead, BinResult, NullString, ReadOptions,
    };
    use serde::{Serialize, Serializer};

    /// A 64 bit file pointer relative to the start of the pointer type.
    #[derive(Serialize, Debug)]
    #[repr(transparent)]
    pub struct RelPtr64<BR: BinRead>(BR);

    impl<BR: BinRead> BinRead for RelPtr64<BR> {
        type Args = BR::Args;

        fn read_options<R: Read + Seek>(
            reader: &mut R,
            options: &ReadOptions,
            args: Self::Args,
        ) -> BinResult<Self> {
            let pos_before_read = reader.seek(SeekFrom::Current(0))?;
            let ptr = u64::read_options(reader, options, ())?;
            let saved_pos = reader.seek(SeekFrom::Current(0))?;

            reader.seek(SeekFrom::Start(pos_before_read + ptr))?;
            let value = BR::read_options(reader, options, args)?;

            reader.seek(SeekFrom::Start(saved_pos))?;

            Ok(Self(value))
        }
    }

    impl<BR: BinRead> core::ops::Deref for RelPtr64<BR> {
        type Target = BR;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[derive(BinRead, Debug)]
    pub struct SsbhString {
        value: RelPtr64<NullString>,
    }

    impl Serialize for SsbhString {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            // TODO: Why doesn't into_string() work?
            let text = &self.value.0;
            let text = std::str::from_utf8(&text).unwrap();
            serializer.serialize_str(&text)
        }
    }

    #[derive(Serialize, Debug)]
    pub struct SsbhArray<T: BinRead<Args = ()>> {
        elements: Vec<T>,
    }

    impl<T> BinRead for SsbhArray<T>
    where
        T: BinRead<Args = ()>,
    {
        type Args = ();

        fn read_options<R: Read + Seek>(
            reader: &mut R,
            options: &ReadOptions,
            _args: Self::Args,
        ) -> BinResult<Self> {
            // TODO: Use relative pointer?
            let pos_before_read = reader.seek(SeekFrom::Current(0))?;
            let ptr = u64::read_options(reader, options, ())?;
            let element_count = u64::read_options(reader, options, ())?;
            let saved_pos = reader.seek(SeekFrom::Current(0))?;

            // TODO: This is a really naive implementation.
            reader.seek(SeekFrom::Start(pos_before_read + ptr))?;
            let mut elements = Vec::new();
            for _i in 0..element_count {
                let element = T::read_options(reader, options, ())?;
                elements.push(element);
            }
            reader.seek(SeekFrom::Start(saved_pos))?;

            Ok(Self { elements })
        }
    }

    #[derive(BinRead, Serialize, Debug)]
    pub struct Vector3 {
        x: f32,
        y: f32,
        z: f32,
    }

    #[derive(BinRead, Serialize, Debug)]
    pub struct Matrix3x3 {
        row1: Vector3,
        row2: Vector3,
        row3: Vector3
    }
}
