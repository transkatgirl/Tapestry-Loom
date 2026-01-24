use jiff::{
    Zoned,
    fmt::temporal::{DateTimeParser, DateTimePrinter},
};
use universal_weave::rkyv::{
    Place, SerializeUnsized,
    rancor::{Fallible, Source},
    string::{ArchivedString, StringResolver},
    with::{ArchiveWith, DeserializeWith, SerializeWith},
};

pub struct AsTemporal;

impl ArchiveWith<Zoned> for AsTemporal {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    #[inline]
    fn resolve_with(field: &Zoned, resolver: Self::Resolver, out: Place<Self::Archived>) {
        // It's safe to unwrap here because if the OsString wasn't valid UTF-8
        // it would have failed to serialize
        ArchivedString::resolve_from_str(
            &DateTimePrinter::new().zoned_to_string(field),
            resolver,
            out,
        );
    }
}

impl<S> SerializeWith<Zoned, S> for AsTemporal
where
    S: Fallible + ?Sized,
    S::Error: Source,
    str: SerializeUnsized<S>,
{
    fn serialize_with(field: &Zoned, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedString::serialize_from_str(
            &DateTimePrinter::new().zoned_to_string(field),
            serializer,
        )
    }
}

impl<D> DeserializeWith<ArchivedString, Zoned, D> for AsTemporal
where
    D: Fallible + ?Sized,
{
    fn deserialize_with(field: &ArchivedString, _: &mut D) -> Result<Zoned, D::Error> {
        Ok(DateTimeParser::new()
            .parse_zoned(field.as_str())
            .unwrap_or_default())
    }
}
