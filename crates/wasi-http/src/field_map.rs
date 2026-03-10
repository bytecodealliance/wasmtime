use http::header::Entry;
use http::{HeaderMap, HeaderName, HeaderValue};
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;
use wasmtime::Result;

/// A wrapper around [`http::HeaderMap`] which implements `wasi:http` semantics.
///
/// The main differences from [`http::HeaderMap`] and this type are:
///
/// * A slimmed down mutability API to just what `wasi:http` needs.
/// * `FieldMap` is cheaply clone-able with the internal `HeaderMap` being
///   behind an `Arc`.
/// * `FieldMap` is either immutable or mutable. Mutations on immutable values
///   are rejected with an error. Mutations on mutable values will never panic
///   unlike `HeaderMap` and additionally require a limit to be set on the size
///   of the map.
///
/// Overall the intention is that this is a slim wrapper around
/// [`http::HeaderMap`] with slightly different ownership, panic, and error
/// semantics.
#[derive(Debug, Clone)]
pub struct FieldMap {
    map: Arc<HeaderMap>,
    limit: Limit,
    size: usize,
}

#[derive(Debug, Clone)]
enum Limit {
    Mutable(usize),
    Immutable,
}

impl Default for FieldMap {
    fn default() -> Self {
        Self::new_immutable(HeaderMap::default())
    }
}

impl FieldMap {
    /// Creates a new immutable `FieldMap` from the provided
    /// [`http::HeaderMap`].
    ///
    /// The returned value cannot be mutated and attempting to mutate it will
    /// return an error.
    pub fn new_immutable(map: HeaderMap) -> Self {
        let size = Self::content_size(&map);
        Self {
            map: Arc::new(map),
            size,
            limit: Limit::Immutable,
        }
    }

    /// Creates a new, empty, mutable `FieldMap`.
    ///
    /// Mutations are allowed on the returned value and up to `limit` bytes of
    /// memory (roughly) may be consumed by this map.
    pub fn new_mutable(limit: usize) -> Self {
        Self {
            map: Arc::new(HeaderMap::new()),
            size: 0,
            limit: Limit::Mutable(limit),
        }
    }

    /// Calculate the content size of a `HeaderMap`. This is a sum of the size
    /// of all of the keys and all of the values.
    pub(crate) fn content_size(map: &HeaderMap) -> usize {
        let mut sum = 0;
        for key in map.keys() {
            sum += header_name_size(key);
        }
        for value in map.values() {
            sum += header_value_size(value);
        }
        sum
    }

    /// Sets the header `key` to the `values` list provided.
    ///
    /// Removes the previous value, if any.
    ///
    /// If `values` is empty then this removes the header `key`.
    //
    // FIXME(WebAssembly/WASI#900): is this the right behavior?
    pub fn set(&mut self, key: HeaderName, values: Vec<HeaderValue>) -> Result<(), FieldMapError> {
        let (map, limit, size) = self.mutable()?;
        let key_size = header_name_size(&key);
        let values_size = values.iter().map(header_value_size).sum::<usize>();
        let mut values = values.into_iter();
        let mut entry = match map.try_entry(key)? {
            Entry::Vacant(e) => match values.next() {
                Some(v) => {
                    update_size(size, limit, *size + values_size + key_size)?;
                    e.try_insert_entry(v)?
                }
                None => return Ok(()),
            },
            Entry::Occupied(mut e) => {
                let prev_values_size = e.iter().map(header_value_size).sum::<usize>();
                let _prev = match values.next() {
                    Some(v) => {
                        update_size(size, limit, *size - prev_values_size + values_size)?;
                        e.insert(v);
                    }
                    None => {
                        update_size(size, limit, *size - prev_values_size - key_size)?;
                        e.remove();
                        return Ok(());
                    }
                };
                e
            }
        };
        for value in values {
            entry.append(value);
        }
        Ok(())
    }

    /// Remove all values associated with a key in a map.
    ///
    /// Returns an empty list if the key is not already present within the map.
    pub fn remove_all(&mut self, key: HeaderName) -> Result<Vec<HeaderValue>, FieldMapError> {
        let (map, _limit, size) = self.mutable()?;
        match map.try_entry(key)? {
            Entry::Vacant { .. } => Ok(Vec::new()),
            Entry::Occupied(e) => {
                let (name, value_drain) = e.remove_entry_mult();
                let mut removed = header_name_size(&name);
                let values = value_drain.collect::<Vec<_>>();
                for v in values.iter() {
                    removed += header_value_size(v);
                }
                *size -= removed;
                Ok(values)
            }
        }
    }

    fn mutable(&mut self) -> Result<(&mut HeaderMap, usize, &mut usize), FieldMapError> {
        match self.limit {
            Limit::Immutable => Err(FieldMapError::Immutable),
            Limit::Mutable(limit) => Ok((Arc::make_mut(&mut self.map), limit, &mut self.size)),
        }
    }

    /// Add a value associated with a key to the map.
    ///
    /// If `key` is already present within the map then `value` is appended to
    /// the list of values it already has.
    pub fn append(&mut self, key: HeaderName, value: HeaderValue) -> Result<bool, FieldMapError> {
        let (map, limit, size) = self.mutable()?;
        let key_size = header_name_size(&key);
        let val_size = header_value_size(&value);
        let new_size = if !map.contains_key(&key) {
            *size + key_size + val_size
        } else {
            *size + val_size
        };
        update_size(size, limit, new_size)?;
        let already_present = map.try_append(key, value)?;
        self.size = new_size;
        Ok(already_present)
    }

    /// Flags this map as mutable, allowing mutations which can allocate as much
    /// as `limit` memory, in bytes, for this entire map (roughly).
    pub fn set_mutable(&mut self, limit: usize) {
        self.limit = Limit::Mutable(limit);
    }

    /// Flags this map as immutable, forbidding all further mutations.
    pub fn set_immutable(&mut self) {
        self.limit = Limit::Immutable;
    }
}

/// Returns the size, in accounting cost, to consider for `name`.
///
/// This includes both the byte length of the `name` itself as well as the size
/// of the data structure itself as it'll reside within a `HeaderMap`.
fn header_name_size(name: &HeaderName) -> usize {
    name.as_str().len() + size_of::<HeaderName>()
}

/// Same as `header_name_size`, but for values.
///
/// This notably includes the size of `HeaderValue` itself to ensure that all
/// headers have a nonzero size as otherwise this would never limit addition of
/// an empty header value.
fn header_value_size(value: &HeaderValue) -> usize {
    value.len() + size_of::<HeaderValue>()
}

fn update_size(size: &mut usize, limit: usize, new: usize) -> Result<(), FieldMapError> {
    if new > limit {
        Err(FieldMapError::TotalSizeTooBig)
    } else {
        *size = new;
        Ok(())
    }
}

// Note that `DerefMut` is specifically omitted here to force all mutations
// through the `FieldMap` wrapper.
impl Deref for FieldMap {
    type Target = HeaderMap;

    fn deref(&self) -> &HeaderMap {
        &self.map
    }
}

impl From<FieldMap> for HeaderMap {
    fn from(map: FieldMap) -> Self {
        Arc::unwrap_or_clone(map.map)
    }
}

/// Errors that can happen when mutating/operating on a [`FieldMap`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FieldMapError {
    /// A mutation was attempted when the map is not mutable.
    Immutable,
    /// The map has too many fields and is not allowed to add more.
    ///
    /// Note that this is currently a limitation inherited from
    /// [`http::HeaderMap`].
    TooManyFields,
    /// The map's total size, of keys and values, is too large.
    TotalSizeTooBig,
    /// An invalid header name was attempted to be added.
    InvalidHeaderName,
}

impl fmt::Display for FieldMapError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            FieldMapError::Immutable => "cannot mutate an immutable field map",
            FieldMapError::TooManyFields => "too many fields in the field map",
            FieldMapError::TotalSizeTooBig => "total size of fields exceeds limit",
            FieldMapError::InvalidHeaderName => "invalid header name",
        };
        f.write_str(s)
    }
}

impl std::error::Error for FieldMapError {}

impl From<http::header::MaxSizeReached> for FieldMapError {
    fn from(_: http::header::MaxSizeReached) -> Self {
        Self::TooManyFields
    }
}

impl From<http::header::InvalidHeaderName> for FieldMapError {
    fn from(_: http::header::InvalidHeaderName) -> Self {
        Self::InvalidHeaderName
    }
}

#[cfg(test)]
mod tests {
    use super::{FieldMap, FieldMapError};

    #[test]
    fn test_immutable() {
        let mut map = FieldMap::default();
        assert_eq!(
            map.set("foo".parse().unwrap(), vec!["bar".parse().unwrap()]),
            Err(FieldMapError::Immutable)
        );
        assert_eq!(
            map.append("foo".parse().unwrap(), "bar".parse().unwrap()),
            Err(FieldMapError::Immutable)
        );
        assert_eq!(
            map.remove_all("foo".parse().unwrap()),
            Err(FieldMapError::Immutable)
        );
    }

    #[test]
    fn test_limits() {
        let mut map = FieldMap::new_mutable(100);
        loop {
            match map.append("foo".parse().unwrap(), "bar".parse().unwrap()) {
                Ok(_) => {}
                Err(FieldMapError::TotalSizeTooBig) => break,
                Err(e) => panic!("unexpected error: {e}"),
            }
        }

        map = FieldMap::new_mutable(100);
        for i in 0.. {
            match map.set(
                "foo".parse().unwrap(),
                (0..i).map(|j| format!("bar{j}").parse().unwrap()).collect(),
            ) {
                Ok(_) => {}
                Err(FieldMapError::TotalSizeTooBig) => break,
                Err(e) => panic!("unexpected error: {e}"),
            }
        }

        map = FieldMap::new_mutable(100);
        for i in 0.. {
            match map.set(
                format!("foo{i}").parse().unwrap(),
                vec!["bar".parse().unwrap()],
            ) {
                Ok(_) => {}
                Err(FieldMapError::TotalSizeTooBig) => break,
                Err(e) => panic!("unexpected error: {e}"),
            }
        }
    }

    #[test]
    fn test_size() -> Result<(), FieldMapError> {
        let mut map = FieldMap::new_mutable(2000);
        let name: http::HeaderName = "foo".parse().unwrap();

        map.append(name.clone(), "bar".parse().unwrap())?;
        assert!(map.size > 0);
        map.remove_all(name.clone())?;
        assert_eq!(map.size, 0);

        map.set(name.clone(), vec!["bar".parse().unwrap()])?;
        assert!(map.size > 0);
        map.remove_all(name.clone())?;
        assert_eq!(map.size, 0);

        map.set(name.clone(), vec![])?;
        assert_eq!(map.size, 0);
        map.set(name.clone(), vec!["bar".parse().unwrap()])?;
        assert!(map.size > 0);
        map.set(name.clone(), vec![])?;
        assert_eq!(map.size, 0);

        map.set(name.clone(), vec!["bar".parse().unwrap()])?;
        assert!(map.size > 0);
        map.set(
            name.clone(),
            vec!["bar".parse().unwrap(), "baz".parse().unwrap()],
        )?;
        assert!(map.size > 0);
        map.remove_all(name.clone())?;
        assert_eq!(map.size, 0);

        Ok(())
    }
}
