#[cfg(feature = "v1")]
use std::{
    collections::{
        HashMap, HashSet,
        hash_map::{Entry, OccupiedEntry},
    },
    hash::{BuildHasher, Hash},
};

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

#[cfg(feature = "v1")]
pub struct UniqueIdentifierRemapper<K, V, KS, VS>
where
    K: Hash + Eq,
    V: Hash + Eq,
    KS: BuildHasher + Default + Clone,
    VS: BuildHasher + Default + Clone,
{
    old_to_new: HashMap<K, V, KS>,
    new_ids: HashSet<V, VS>,
}

#[cfg(feature = "v1")]
impl<K, V, KS, VS> UniqueIdentifierRemapper<K, V, KS, VS>
where
    K: Hash + Eq,
    V: Hash + Clone + Eq,
    KS: BuildHasher + Default + Clone,
    VS: BuildHasher + Default + Clone,
{
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            old_to_new: HashMap::with_capacity_and_hasher(capacity, KS::default()),
            new_ids: HashSet::with_capacity_and_hasher(capacity, VS::default()),
        }
    }
    pub fn extend_output_id_set(&mut self, iter: impl Iterator<Item = V>) {
        self.new_ids.extend(iter);
    }
    pub fn map(&mut self, input: K, mut generator: impl FnMut() -> V) -> OccupiedEntry<'_, K, V> {
        let mut generate_unique = || {
            let mut id = generator();

            while self.new_ids.contains(&id) {
                id = generator();
            }

            self.new_ids.insert(id.clone());

            id
        };

        match self.old_to_new.entry(input) {
            Entry::Occupied(occupied) => occupied,
            Entry::Vacant(vacant) => vacant.insert_entry(generate_unique()),
        }
    }
    pub fn map_with_initial(
        &mut self,
        input: K,
        initial: V,
        mut generator: impl FnMut() -> V,
    ) -> OccupiedEntry<'_, K, V> {
        let generate_unique = || {
            let mut id = initial;

            while self.new_ids.contains(&id) {
                id = generator();
            }

            self.new_ids.insert(id.clone());

            id
        };

        match self.old_to_new.entry(input) {
            Entry::Occupied(occupied) => occupied,
            Entry::Vacant(vacant) => vacant.insert_entry(generate_unique()),
        }
    }
    pub fn try_map<E>(
        &mut self,
        input: K,
        mut generator: impl FnMut() -> Result<V, E>,
    ) -> Result<OccupiedEntry<'_, K, V>, E> {
        let mut generate_unique = || {
            let mut id = generator()?;

            while self.new_ids.contains(&id) {
                id = generator()?;
            }

            self.new_ids.insert(id.clone());

            Ok(id)
        };

        match self.old_to_new.entry(input) {
            Entry::Occupied(occupied) => Ok(occupied),
            Entry::Vacant(vacant) => Ok(vacant.insert_entry(generate_unique()?)),
        }
    }
    pub fn try_map_with_initial<E>(
        &mut self,
        input: K,
        initial: V,
        mut generator: impl FnMut() -> Result<V, E>,
    ) -> Result<OccupiedEntry<'_, K, V>, E> {
        let generate_unique = || {
            let mut id = initial;

            while self.new_ids.contains(&id) {
                id = generator()?;
            }

            self.new_ids.insert(id.clone());

            Ok(id)
        };

        match self.old_to_new.entry(input) {
            Entry::Occupied(occupied) => Ok(occupied),
            Entry::Vacant(vacant) => Ok(vacant.insert_entry(generate_unique()?)),
        }
    }
}
