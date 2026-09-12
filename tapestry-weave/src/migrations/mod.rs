use std::{fmt::Display, io::Write};

use rkyv::{
    access,
    api::high::to_bytes_in,
    from_bytes,
    rancor::{Error, Source},
    ser::{Writer, writer::IoWriter},
};
use universal_weave::versioning::VersionedBytes;

use crate::{
    HEADER_MAGIC_BYTES,
    weave::{ArchivedTapestryWeave, ArchivedTapestryWeaveInner, TapestryWeave, TapestryWeaveInner},
};

#[cfg(feature = "v0")]
mod v0;

impl TapestryWeave {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 32 {
            return Err(Error::new(HeaderError::TooShort));
        }

        if let Some(versioned) = VersionedBytes::try_from_bytes(bytes, HEADER_MAGIC_BYTES) {
            match versioned.version {
                #[cfg(feature = "v0")]
                v0::FORMAT_VERSION => {
                    from_bytes::<v0::TapestryWeave, Error>(versioned.data).map(|weave| weave.into())
                }
                super::weave::FORMAT_VERSION => {
                    from_bytes::<TapestryWeaveInner, Error>(versioned.data)
                        .map(|weave| weave.into())
                }
                _ => Err(Error::new(HeaderError::UnsupportedVersion)),
            }
        } else {
            Err(Error::new(HeaderError::BadHeader))
        }
    }
    pub fn write_bytes<W: Write>(&self, writer: W) -> Result<(), Error> {
        let mut writer = IoWriter::new(writer);

        // Copied from VersionedBytes::write_header()
        writer.write(&HEADER_MAGIC_BYTES)?;
        writer.write(&super::weave::FORMAT_VERSION.to_le_bytes())?;

        assert!(self.weave.validate());
        to_bytes_in::<IoWriter<W>, Error>(&self.weave, writer)?;

        Ok(())
    }
}

impl<'a> ArchivedTapestryWeave<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() < 32 {
            return Err(Error::new(HeaderError::TooShort));
        }

        if let Some(versioned) = VersionedBytes::try_from_bytes(bytes, HEADER_MAGIC_BYTES) {
            if versioned.version == super::weave::FORMAT_VERSION {
                access::<ArchivedTapestryWeaveInner, Error>(versioned.data)
                    .map(|weave| Self { weave })
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
