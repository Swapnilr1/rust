use std::marker::PhantomData;

use crate::Idx;

/// A map from arena indexes to some other type.
/// Space requirement is O(highest index).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArenaMap<IDX, V> {
    v: Vec<Option<V>>,
    _ty: PhantomData<IDX>,
}

impl<T, V> ArenaMap<Idx<T>, V> {
    /// Creates a new empty map.
    pub const fn new() -> Self {
        Self { v: Vec::new(), _ty: PhantomData }
    }

    /// Create a new empty map with specific capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self { v: Vec::with_capacity(capacity), _ty: PhantomData }
    }

    /// Inserts a value associated with a given arena index into the map.
    pub fn insert(&mut self, idx: Idx<T>, t: V) {
        let idx = Self::to_idx(idx);

        self.v.resize_with((idx + 1).max(self.v.len()), || None);
        self.v[idx] = Some(t);
    }

    /// Returns a reference to the value associated with the provided index
    /// if it is present.
    pub fn get(&self, idx: Idx<T>) -> Option<&V> {
        self.v.get(Self::to_idx(idx)).and_then(|it| it.as_ref())
    }

    /// Returns a mutable reference to the value associated with the provided index
    /// if it is present.
    pub fn get_mut(&mut self, idx: Idx<T>) -> Option<&mut V> {
        self.v.get_mut(Self::to_idx(idx)).and_then(|it| it.as_mut())
    }

    /// Returns an iterator over the values in the map.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.v.iter().filter_map(|o| o.as_ref())
    }

    /// Returns an iterator over mutable references to the values in the map.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.v.iter_mut().filter_map(|o| o.as_mut())
    }

    /// Returns an iterator over the arena indexes and values in the map.
    pub fn iter(&self) -> impl Iterator<Item = (Idx<T>, &V)> {
        self.v.iter().enumerate().filter_map(|(idx, o)| Some((Self::from_idx(idx), o.as_ref()?)))
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    pub fn entry(&mut self, idx: Idx<T>) -> Entry<'_, Idx<T>, V> {
        let idx = Self::to_idx(idx);
        self.v.resize_with((idx + 1).max(self.v.len()), || None);
        match &mut self.v[idx] {
            slot @ Some(_) => Entry::Occupied(OccupiedEntry { slot, _ty: PhantomData }),
            slot @ None => Entry::Vacant(VacantEntry { slot, _ty: PhantomData }),
        }
    }

    fn to_idx(idx: Idx<T>) -> usize {
        u32::from(idx.into_raw()) as usize
    }

    fn from_idx(idx: usize) -> Idx<T> {
        Idx::from_raw((idx as u32).into())
    }
}

impl<T, V> std::ops::Index<Idx<V>> for ArenaMap<Idx<V>, T> {
    type Output = T;
    fn index(&self, idx: Idx<V>) -> &T {
        self.v[Self::to_idx(idx)].as_ref().unwrap()
    }
}

impl<T, V> std::ops::IndexMut<Idx<V>> for ArenaMap<Idx<V>, T> {
    fn index_mut(&mut self, idx: Idx<V>) -> &mut T {
        self.v[Self::to_idx(idx)].as_mut().unwrap()
    }
}

impl<T, V> Default for ArenaMap<Idx<V>, T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A view into a single entry in a map, which may either be vacant or occupied.
///
/// This `enum` is constructed from the [`entry`] method on [`ArenaMap`].
///
/// [`entry`]: ArenaMap::entry
pub enum Entry<'a, IDX, V> {
    /// A vacant entry.
    Vacant(VacantEntry<'a, IDX, V>),
    /// An occupied entry.
    Occupied(OccupiedEntry<'a, IDX, V>),
}

impl<'a, IDX, V> Entry<'a, IDX, V> {
    /// Ensures a value is in the entry by inserting the default if empty, and returns a mutable reference to
    /// the value in the entry.
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Self::Vacant(ent) => ent.insert(default),
            Self::Occupied(ent) => ent.into_mut(),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the default function if empty, and returns
    /// a mutable reference to the value in the entry.
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Self::Vacant(ent) => ent.insert(default()),
            Self::Occupied(ent) => ent.into_mut(),
        }
    }

    /// Provides in-place mutable access to an occupied entry before any potential inserts into the map.
    pub fn and_modify<F: FnOnce(&mut V)>(mut self, f: F) -> Self {
        if let Self::Occupied(ent) = &mut self {
            f(ent.get_mut());
        }
        self
    }
}

impl<'a, IDX, V> Entry<'a, IDX, V>
where
    V: Default,
{
    /// Ensures a value is in the entry by inserting the default value if empty, and returns a mutable reference
    /// to the value in the entry.
    pub fn or_default(self) -> &'a mut V {
        self.or_insert_with(Default::default)
    }
}

/// A view into an vacant entry in a [`ArenaMap`]. It is part of the [`Entry`] enum.
pub struct VacantEntry<'a, IDX, V> {
    slot: &'a mut Option<V>,
    _ty: PhantomData<IDX>,
}

impl<'a, IDX, V> VacantEntry<'a, IDX, V> {
    /// Sets the value of the entry with the `VacantEntry`’s key, and returns a mutable reference to it.
    pub fn insert(self, value: V) -> &'a mut V {
        self.slot.insert(value)
    }
}

/// A view into an occupied entry in a [`ArenaMap`]. It is part of the [`Entry`] enum.
pub struct OccupiedEntry<'a, IDX, V> {
    slot: &'a mut Option<V>,
    _ty: PhantomData<IDX>,
}

impl<'a, IDX, V> OccupiedEntry<'a, IDX, V> {
    /// Gets a reference to the value in the entry.
    pub fn get(&self) -> &V {
        self.slot.as_ref().expect("Occupied")
    }

    /// Gets a mutable reference to the value in the entry.
    pub fn get_mut(&mut self) -> &mut V {
        self.slot.as_mut().expect("Occupied")
    }

    /// Converts the entry into a mutable reference to its value.
    pub fn into_mut(self) -> &'a mut V {
        self.slot.as_mut().expect("Occupied")
    }

    /// Sets the value of the entry with the `OccupiedEntry`’s key, and returns the entry’s old value.
    pub fn insert(&mut self, value: V) -> V {
        self.slot.replace(value).expect("Occupied")
    }

    /// Takes the value of the entry out of the map, and returns it.
    pub fn remove(self) -> V {
        self.slot.take().expect("Occupied")
    }
}
