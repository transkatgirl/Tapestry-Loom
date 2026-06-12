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

#[cfg(feature = "v1")]
const PRINTER: DateTimePrinter = DateTimePrinter::new();

#[cfg(feature = "v1")]
const PARSER: DateTimeParser = DateTimeParser::new();

#[cfg(feature = "v1")]
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct BinaryZoned {
    secs: i64,
    nanos: i32,
    timezone: String,
}

#[cfg(feature = "v1")]
impl From<&Zoned> for BinaryZoned {
    fn from(value: &Zoned) -> Self {
        let timezone = PRINTER
            .time_zone_to_string(value.time_zone())
            .unwrap_or_else(|_| "Etc/Unknown".to_string());
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
        let timezone = PARSER
            .parse_time_zone(value.timezone)
            .unwrap_or(TimeZone::unknown());
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
