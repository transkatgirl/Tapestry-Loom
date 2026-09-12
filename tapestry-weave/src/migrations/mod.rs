use std::{fmt::Display, io::Write};

use serde::{Deserialize, Serialize, de::Error as DeError};
use serde_json::{Value, from_str, from_value, to_string, to_value};
use universal_weave::{
    rkyv::{
        access,
        api::high::to_bytes_in,
        from_bytes,
        rancor::{Error, Source},
        ser::{Writer, writer::IoWriter},
    },
    versioning::VersionedBytes,
};

use crate::{
    HEADER_MAGIC_BYTES,
    weave::{ArchivedTapestryWeave, ArchivedTapestryWeaveInner, TapestryWeave, TapestryWeaveInner},
};

#[cfg(feature = "v0")]
mod v0;

#[derive(Serialize, Deserialize, Debug)]
struct VersionedJson {
    version: u64,
    data: Value,
}

impl TapestryWeave {
    pub fn is_header_valid(bytes: &[u8]) -> bool {
        bytes.starts_with(&HEADER_MAGIC_BYTES) && bytes.len() >= 32
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 32 {
            return Err(Error::new(HeaderError::TooShort));
        }

        if let Some(versioned) = VersionedBytes::try_from_bytes(bytes, HEADER_MAGIC_BYTES) {
            match versioned.version {
                #[cfg(feature = "v0")]
                v0::FORMAT_VERSION => {
                    from_bytes::<v0::TapestryWeave, Error>(versioned.data).map(Self::from)
                }
                super::weave::FORMAT_VERSION => {
                    from_bytes::<TapestryWeaveInner, Error>(versioned.data).map(Self::from)
                }
                _ => Err(Error::new(HeaderError::UnsupportedVersion)),
            }
        } else {
            Err(Error::new(HeaderError::BadHeader))
        }
    }
    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        let versioned = from_str::<VersionedJson>(json)?;

        match versioned.version {
            #[cfg(feature = "v0")]
            v0::FORMAT_VERSION => {
                from_value::<v0::TapestryWeave>(versioned.data).map(|weave| weave.into())
            }
            super::weave::FORMAT_VERSION => {
                from_value::<TapestryWeaveInner>(versioned.data).map(|weave| weave.into())
            }
            _ => Err(serde_json::Error::custom("unsupported version")),
        }
    }
    pub fn to_bytes_in<W: Write>(&self, writer: W) -> Result<(), Error> {
        let mut writer = IoWriter::new(writer);

        // Copied from VersionedBytes::write_header()
        writer.write(&HEADER_MAGIC_BYTES)?;
        writer.write(&super::weave::FORMAT_VERSION.to_le_bytes())?;

        assert!(self.as_ref().validate());
        to_bytes_in::<IoWriter<W>, Error>(self.as_ref(), writer)?;

        Ok(())
    }
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        assert!(self.as_ref().validate());

        to_string(&VersionedJson {
            version: super::weave::FORMAT_VERSION,
            data: to_value(self.as_ref())?,
        })
    }
}

impl<'a> ArchivedTapestryWeave<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() < 32 {
            return Err(Error::new(HeaderError::TooShort));
        }

        if let Some(versioned) = VersionedBytes::try_from_bytes(bytes, HEADER_MAGIC_BYTES) {
            if versioned.version == super::weave::FORMAT_VERSION {
                access::<ArchivedTapestryWeaveInner, Error>(versioned.data).map(Self::from)
            } else {
                Err(Error::new(HeaderError::UnsupportedVersion))
            }
        } else {
            Err(Error::new(HeaderError::BadHeader))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use]
enum HeaderError {
    UnsupportedVersion,
    BadHeader,
    TooShort,
}

impl Display for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeaderError::UnsupportedVersion => {
                write!(f, "unsupported version")
            }
            HeaderError::BadHeader => {
                write!(f, "header contains incorrect magic bytes")
            }
            HeaderError::TooShort => {
                write!(f, "input is too short")
            }
        }
    }
}

impl std::error::Error for HeaderError {}
