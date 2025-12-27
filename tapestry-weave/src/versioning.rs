use universal_weave::rkyv::util::AlignedVec;

const FORMAT_IDENTIFIER: &[u8] = b"VersionedTapestryWeave__";
const FORMAT_IDENTIFIER_LENGTH: usize = FORMAT_IDENTIFIER.len();
const VERSION_LENGTH: usize = 8;
const DATA_OFFSET: usize = FORMAT_IDENTIFIER_LENGTH + VERSION_LENGTH;

pub struct VersionedBytes<'a> {
    pub version: u64,
    pub data: MixedData<'a>,
}

pub enum MixedData<'a> {
    Input(&'a [u8]),
    Output(AlignedVec),
}

impl AsRef<[u8]> for MixedData<'_> {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Input(b) => b,
            Self::Output(b) => b,
        }
    }
}

impl<'a> VersionedBytes<'a> {
    pub fn from_bytes(value: &'a [u8]) -> Option<Self> {
        if value.starts_with(FORMAT_IDENTIFIER) && value.len() >= DATA_OFFSET {
            let (version_bytes, data) =
                value[FORMAT_IDENTIFIER_LENGTH..].split_at(size_of::<u64>());

            Some(Self {
                version: u64::from_le_bytes(version_bytes.try_into().unwrap()),
                data: MixedData::Input(data),
            })
        } else {
            None
        }
    }
    pub fn to_byte_set(self) -> (&'static [u8], [u8; 8], MixedData<'a>) {
        (FORMAT_IDENTIFIER, self.version.to_le_bytes(), self.data)
    }
    pub fn to_bytes(self) -> Vec<u8> {
        let byte_set = self.to_byte_set();

        byte_set
            .0
            .iter()
            .copied()
            .chain(byte_set.1.into_iter().chain(match byte_set.2 {
                MixedData::Input(b) => b.to_owned(),
                MixedData::Output(b) => b.to_vec(),
            }))
            .collect()
    }
}
