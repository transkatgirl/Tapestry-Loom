//! Useful utilities.

use std::{
    collections::{
        HashMap, HashSet,
        hash_map::{Entry, OccupiedEntry},
    },
    hash::{BuildHasher, Hash, Hasher},
    ops::Range,
    time::Instant,
};

use base64::engine::general_purpose::STANDARD;
use base64_serde::base64_serde_type;
use jiff::{
    Timestamp, Zoned,
    fmt::temporal::{DateTimeParser, DateTimePrinter},
    tz::TimeZone,
};
use similar::{Algorithm, DiffTag, capture_diff_slices_deadline};
use universal_weave::{
    indexmap::{IndexMap, IndexSet},
    rkyv::{
        Archive, Deserialize, Place, Resolver, Serialize, SerializeUnsized,
        collections::util,
        rancor::{Fallible, Source},
        vec::{ArchivedVec, VecResolver},
        with::{ArchiveWith, DeserializeWith, SerializeWith},
    },
};

pub use foldhash::fast::RandomState;
pub use rkyv::util::AlignedVec;

/// A [`Hasher`] implementation which passes through values without modification, intended for sets and maps keyed by randomly generated identifiers.
///
/// # Panics
///
/// Panics if methods other than [`Hasher::write_u64`] or [`Hasher::write_u128`] are called.
#[derive(Default)]
pub struct RandomIdHasher(u64);

impl Hasher for RandomIdHasher {
    fn write(&mut self, _: &[u8]) {
        unimplemented!()
    }

    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }

    fn write_u128(&mut self, i: u128) {
        self.0 = i as u64;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

const PRINTER: DateTimePrinter = DateTimePrinter::new();

const PARSER: DateTimeParser = DateTimeParser::new();

base64_serde_type!(pub(crate) Base64Standard, STANDARD);

pub(crate) struct IAsVec;

impl<K: Archive, V: Archive, H> ArchiveWith<IndexMap<K, V, H>> for IAsVec {
    type Archived = ArchivedVec<util::Entry<K::Archived, V::Archived>>;
    type Resolver = VecResolver;

    fn resolve_with(
        field: &IndexMap<K, V, H>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_len(field.len(), resolver, out);
    }
}

impl<K, V, H, S> SerializeWith<IndexMap<K, V, H>, S> for IAsVec
where
    K: Serialize<S>,
    V: Serialize<S>,
    S: Fallible + rkyv::ser::Allocator + rkyv::ser::Writer + ?Sized,
{
    fn serialize_with(
        field: &IndexMap<K, V, H>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_iter(
            field
                .iter()
                .map(|(key, value)| util::EntryAdapter::<_, _, K, V>::new(key, value)),
            serializer,
        )
    }
}

impl<K, V, H, D>
    DeserializeWith<ArchivedVec<util::Entry<K::Archived, V::Archived>>, IndexMap<K, V, H>, D>
    for IAsVec
where
    K: Archive + Hash + Eq,
    V: Archive,
    K::Archived: Deserialize<K, D>,
    V::Archived: Deserialize<V, D>,
    H: BuildHasher + Default,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedVec<util::Entry<K::Archived, V::Archived>>,
        deserializer: &mut D,
    ) -> Result<IndexMap<K, V, H>, D::Error> {
        let mut result = IndexMap::with_capacity_and_hasher(field.len(), H::default());
        for entry in field.iter() {
            result.insert(
                entry.key.deserialize(deserializer)?,
                entry.value.deserialize(deserializer)?,
            );
        }
        Ok(result)
    }
}

impl<T: Archive, H> ArchiveWith<IndexSet<T, H>> for IAsVec {
    type Archived = ArchivedVec<T::Archived>;
    type Resolver = VecResolver;

    fn resolve_with(field: &IndexSet<T, H>, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedVec::resolve_from_len(field.len(), resolver, out);
    }
}

impl<T, H, S> SerializeWith<IndexSet<T, H>, S> for IAsVec
where
    T: Serialize<S>,
    S: Fallible + rkyv::ser::Allocator + rkyv::ser::Writer + ?Sized,
{
    fn serialize_with(
        field: &IndexSet<T, H>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::<T::Archived>::serialize_from_iter::<T, _, _>(field.iter(), serializer)
    }
}

impl<T, H, D> DeserializeWith<ArchivedVec<T::Archived>, IndexSet<T, H>, D> for IAsVec
where
    T: Archive + Hash + Eq,
    T::Archived: Deserialize<T, D>,
    H: BuildHasher + Default,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedVec<T::Archived>,
        deserializer: &mut D,
    ) -> Result<IndexSet<T, H>, D::Error> {
        let mut result = IndexSet::with_capacity_and_hasher(field.len(), H::default());
        for key in field.iter() {
            result.insert(key.deserialize(deserializer)?);
        }
        Ok(result)
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct BinaryZoned {
    pub(crate) secs: i64,
    pub(crate) nanos: i32,
    pub(crate) timezone: String, // Timezones are usually IANA names, so wasting bytes when storing offsets for POSIX timestamps is probably fine.
}

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

impl From<BinaryZoned> for Zoned {
    fn from(value: BinaryZoned) -> Self {
        let timezone = if value.timezone.is_empty() {
            TimeZone::UTC
        } else {
            PARSER
                .parse_time_zone(value.timezone)
                .unwrap_or(TimeZone::unknown())
        };
        let timestamp = Timestamp::new(value.secs, value.nanos).unwrap_or_default();

        timestamp.to_zoned(timezone)
    }
}

pub(crate) struct AsBinaryZoned;

impl ArchiveWith<Zoned> for AsBinaryZoned {
    type Archived = ArchivedBinaryZoned;
    type Resolver = Resolver<BinaryZoned>;

    #[inline]
    fn resolve_with(field: &Zoned, resolver: Self::Resolver, out: Place<Self::Archived>) {
        BinaryZoned::from(field).resolve(resolver, out);
    }
}

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

/// A mapper for converting between two different types of identifiers.
///
/// Mappings are guaranteed to be unique; No two inputs are mapped to the same output.
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

impl<K, V, KS, VS> Default for UniqueIdentifierRemapper<K, V, KS, VS>
where
    K: Hash + Eq,
    V: Hash + Clone + Eq,
    KS: BuildHasher + Default + Clone,
    VS: BuildHasher + Default + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, KS, VS> UniqueIdentifierRemapper<K, V, KS, VS>
where
    K: Hash + Eq,
    V: Hash + Clone + Eq,
    KS: BuildHasher + Default + Clone,
    VS: BuildHasher + Default + Clone,
{
    /// Creates a new, empty [`UniqueIdentifierRemapper`].
    pub fn new() -> Self {
        Self {
            old_to_new: HashMap::with_hasher(KS::default()),
            new_ids: HashSet::with_hasher(VS::default()),
        }
    }
    /// Creates a new, empty [`UniqueIdentifierRemapper`] with at least the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            old_to_new: HashMap::with_capacity_and_hasher(capacity, KS::default()),
            new_ids: HashSet::with_capacity_and_hasher(capacity, VS::default()),
        }
    }
    /// Extends the set of used identifiers with the following iterator.
    ///
    /// Identifiers consumed by this iterator which were not already used as a mapping value will never be returned during mapping.
    pub fn extend_taken_id_set(&mut self, iter: impl Iterator<Item = V>) {
        self.new_ids.extend(iter);
    }
    /// Converts the mapper into a [`HashMap`] between the old and new identifier types.
    pub fn into_map(self) -> HashMap<K, V, KS> {
        self.old_to_new
    }
    /// Maps the input identifier to its corresponding output identifier.
    ///
    /// If the input has not already been mapped, a unique identifier will be created using `generator`.
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
    /// Maps the input identifier to its corresponding output identifier.
    ///
    /// If the input has not already been mapped, `initial` will be used as the output if it has not already been used by a mapping. If `initial` has been used previously, a unique identifier will be created using `generator`.
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
}

/// A contiguous region of difference between an old and a new sequence.
///
/// Hunks are applied by replacing `old[hunk.old]` with `new[hunk.new]`. Either range may be empty.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[must_use]
pub struct Hunk {
    /// The range of the old sequence which is replaced.
    pub old: Range<usize>,
    /// The range of the new sequence which replaces [`Self::old`].
    pub new: Range<usize>,
}

impl Hunk {
    /// Calculates the hunks which transform `old` into `new` by `deadline`.
    ///
    /// Returned hunks are sorted by position, non-overlapping, and the content between consecutive hunks is identical in both sequences.
    pub fn calculate_diff<T>(old: &[T], new: &[T], deadline: Option<Instant>) -> Vec<Self>
    where
        T: Eq + Hash,
    {
        capture_diff_slices_deadline(Algorithm::Myers, old, new, deadline)
            .into_iter()
            .filter_map(|op| {
                let (tag, old, new) = op.as_tag_tuple();

                if tag == DiffTag::Equal {
                    None
                } else {
                    Some(Self { old, new })
                }
            })
            .collect()
    }
    pub(crate) fn expand_ordered(
        hunks: &mut Vec<Self>,
        old_len: usize,
        delta: impl Fn(&Self) -> (usize, usize),
    ) {
        let mut last_end = 0;

        for index in 0..hunks.len() {
            let next_start = hunks.get(index + 1).map_or(old_len, |next| next.old.start);
            let hunk = &mut hunks[index];
            let original_old_end = hunk.old.end;

            loop {
                let (mut left, mut right) = delta(hunk);
                left = left.min(hunk.old.start - last_end);
                right = right.min(next_start - hunk.old.end);

                if left == 0 && right == 0 {
                    break;
                }

                hunk.old.start -= left;
                hunk.new.start -= left;
                hunk.old.end += right;
                hunk.new.end += right;
            }

            last_end = original_old_end;
        }

        hunks.dedup_by(|next, previous| {
            if next.old.start <= previous.old.end {
                previous.old.end = next.old.end;
                previous.new.end = next.new.end;

                true
            } else {
                false
            }
        });
    }
}
