#[cfg(feature = "v1")]
use universal_weave::rkyv::{
    Archive, Deserialize, Place, Resolver, Serialize, SerializeUnsized,
    rancor::{Fallible, Source},
    with::{ArchiveWith, DeserializeWith, SerializeWith},
};

#[cfg(feature = "v1")]
use jiff::{
    SignedDuration, Timestamp, Zoned,
    fmt::temporal::{DateTimeParser, DateTimePrinter},
    tz::TimeZone,
};

#[cfg(feature = "serde")]
use base64::engine::general_purpose::STANDARD;

#[cfg(feature = "serde")]
use base64_serde::base64_serde_type;

#[cfg(feature = "v1")]
const PRINTER: DateTimePrinter = DateTimePrinter::new();

#[cfg(feature = "v1")]
const PARSER: DateTimeParser = DateTimeParser::new();

#[cfg(feature = "serde")]
base64_serde_type!(pub(crate) Base64Standard, STANDARD);

#[cfg(feature = "v1")]
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct BinaryZoned {
    pub secs: i64,
    pub nanos: i32,
    pub timezone: String, // Timezones are usually IANA names, so wasting bytes when storing offsets or POSIX timestamps is probably fine.
}

#[cfg(feature = "v1")]
impl From<&Zoned> for BinaryZoned {
    fn from(value: &Zoned) -> Self {
        let timezone = {
            let timezone = value.time_zone();

            if *timezone == TimeZone::UTC {
                String::new()
            } else {
                PRINTER
                    .time_zone_to_string(timezone)
                    .unwrap_or_else(|_| "Etc/Unknown".to_string())
            }
        };
        let timestamp = value.timestamp().as_duration();

        Self {
            secs: timestamp.as_secs(),
            nanos: timestamp.subsec_nanos(),
            timezone,
        }
    }
}

#[cfg(feature = "v1")]
impl From<BinaryZoned> for Zoned {
    fn from(value: BinaryZoned) -> Self {
        let timezone = if value.timezone.is_empty() {
            TimeZone::UTC
        } else {
            PARSER
                .parse_time_zone(value.timezone)
                .unwrap_or(TimeZone::unknown())
        };
        let timestamp = Timestamp::from_duration(SignedDuration::new(value.secs, value.nanos))
            .unwrap_or_default();

        timestamp.to_zoned(timezone)
    }
}

#[cfg(feature = "v1")]
pub struct AsBinaryZoned;

#[cfg(feature = "v1")]
impl ArchiveWith<Zoned> for AsBinaryZoned {
    type Archived = ArchivedBinaryZoned;
    type Resolver = Resolver<BinaryZoned>;

    #[inline]
    fn resolve_with(field: &Zoned, resolver: Self::Resolver, out: Place<Self::Archived>) {
        BinaryZoned::from(field).resolve(resolver, out);
    }
}

#[cfg(feature = "v1")]
impl<S> SerializeWith<Zoned, S> for AsBinaryZoned
where
    S: Fallible + ?Sized,
    S::Error: Source,
    str: SerializeUnsized<S>,
{
    fn serialize_with(field: &Zoned, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        BinaryZoned::from(field).serialize(serializer)
    }
}

#[cfg(feature = "v1")]
impl<D> DeserializeWith<ArchivedBinaryZoned, Zoned, D> for AsBinaryZoned
where
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedBinaryZoned,
        deserializer: &mut D,
    ) -> Result<Zoned, D::Error> {
        Ok(Zoned::from(field.deserialize(deserializer)?))
    }
}
