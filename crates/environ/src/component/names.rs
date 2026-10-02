use crate::collections::TryCow;
use crate::error::{Result, bail};
use crate::{Atom, StringPool, prelude::*};
use core::borrow::Borrow;
use core::hash::Hash;
use semver::Version;
use serde_derive::{Deserialize, Serialize};
use wasmparser::WasmFeatures;
use wasmparser::names::{
    ComponentName, ComponentNameKind, is_canonical_version, split_canonical_version,
};

/// A semver-aware map for imports/exports of a component.
///
/// This data structure is used when looking up the names of imports/exports of
/// a component to enable semver-compatible matching of lookups. This will
/// enable lookups of `a:b/c@0.2.0` to match entries defined as `a:b/c@0.2.1`
/// which is currently considered a key feature of WASI's compatibility story.
///
/// This map has at most one definition per semver track. Definitions are keyed
/// by their canonical name, see [`canonical_name`], so for example
/// `a:b/c@0.2.0`, `a:b/c@0.2.1`, and `a:b/c@0.2` all refer to the same
/// definition.
///
/// On the outside this looks like a map of `K` to `V`.
#[derive(Serialize, Deserialize, Debug)]
pub struct NameMap<K, V>
where
    K: TryClone + Hash + Eq + Ord,
{
    /// A map of canonical names to the name that each definition was defined
    /// with, and the definition itself.
    definitions: TryIndexMap<K, (K, V)>,
}

impl<K, V> TryClone for NameMap<K, V>
where
    K: TryClone + Hash + Eq + Ord,
    V: TryClone,
{
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        Ok(Self {
            definitions: self.definitions.try_clone()?,
        })
    }
}

impl<K, V> NameMap<K, V>
where
    K: TryClone + Hash + Eq + Ord,
{
    /// Inserts the `name` specified into this map.
    ///
    /// The name is intern'd through the `cx` argument and shadowing is
    /// controlled by the `allow_shadowing` variable.
    ///
    /// Returns an error if `name` isn't a valid component name, see
    /// [`validate_name`], or if `allow_shadowing` is `false` and a definition
    /// on the same semver track as `name` is already present in this map.
    pub fn insert<I>(
        &mut self,
        name: &str,
        cx: &mut I,
        allow_shadowing: bool,
        item: V,
    ) -> Result<()>
    where
        I: NameMapIntern<Key = K>,
        I::BorrowedKey: Hash + Eq,
    {
        validate_name(name)?;
        let key = cx.intern(canonical_name(name))?;
        let name_key = cx.intern(name)?;
        if !allow_shadowing && let Some((prev, _)) = self.definitions.get(&key) {
            if *prev == name_key {
                bail!("map entry `{name}` defined twice")
            }
            bail!("map entry `{name}` is on the same semver track as an existing entry")
        }
        self.definitions.insert(key, (name_key, item))?;
        Ok(())
    }

    /// Looks up `name` within this map, using the interning specified by
    /// `cx`.
    ///
    /// This returns the definition on the semver track of `name`, if any. For
    /// example looking up `a:b/c@0.2.0` or `a:b/c@0.2` returns the definition
    /// of `a:b/c@0.2.1`.
    pub fn get<I>(&self, name: &str, cx: &I) -> Option<&V>
    where
        I: NameMapIntern<Key = K>,
        I::Key: Borrow<I::BorrowedKey>,
        I::BorrowedKey: Hash + Eq,
    {
        let key = cx.lookup(canonical_name(name))?;
        let (_name, item) = self.definitions.get(&*key)?;
        Some(item)
    }

    /// Looks up `name` like [`NameMap::get`] and returns the definition found
    /// if `can_merge` returns `true` for it, and otherwise inserts `default()`
    /// like [`NameMap::insert`].
    ///
    /// This is used to merge definitions on the same semver track. A merged
    /// definition keeps the name it was originally defined with.
    ///
    /// Returns an error if `name` isn't valid, see [`validate_name`], even if
    /// it would otherwise resolve to an existing definition.
    pub fn get_or_insert_with<I>(
        &mut self,
        name: &str,
        cx: &mut I,
        allow_shadowing: bool,
        can_merge: impl FnOnce(&V) -> bool,
        default: impl FnOnce() -> V,
    ) -> Result<&mut V>
    where
        I: NameMapIntern<Key = K>,
        I::BorrowedKey: Hash + Eq,
    {
        validate_name(name)?;
        let key = cx.intern(canonical_name(name))?;
        let merge = matches!(self.definitions.get(&key), Some((_, item)) if can_merge(item));
        if !merge {
            self.insert(name, cx, allow_shadowing, default())?;
        }
        Ok(&mut self.definitions.get_mut(&key).unwrap().1)
    }

    /// Returns an iterator over inserted values in this map.
    ///
    /// Note that the iterator return yields intern'd keys, which are the names
    /// that definitions were inserted with.
    pub fn raw_iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.definitions.values().map(|(name, item)| (name, item))
    }
}

impl<V> NameMap<TryString, V> {
    /// Inserts `name` like [`NameMap::insert`] without shadowing, except that
    /// if a definition with a different name on the same semver track is
    /// already present then only the definition with the higher version is
    /// kept.
    ///
    /// This is used for the exports of a component which, unlike a linker, may
    /// define multiple versions on one semver track.
    pub fn insert_highest(&mut self, name: &str, item: V) -> Result<()> {
        let shadow = match self.definitions.get(canonical_name(name)) {
            Some((prev, _)) if **prev != *name => {
                if full_version(name) < full_version(prev) {
                    return Ok(());
                }
                true
            }
            _ => false,
        };
        self.insert(name, &mut NameMapNoIntern, shadow, item)
    }
}

impl<K, V> Default for NameMap<K, V>
where
    K: TryClone + Hash + Eq + Ord,
{
    fn default() -> NameMap<K, V> {
        NameMap {
            definitions: Default::default(),
        }
    }
}

/// A helper trait used in conjunction with [`NameMap`] to optionally intern
/// keys to non-strings.
pub trait NameMapIntern {
    /// The key that this interning context generates.
    type Key: Borrow<Self::BorrowedKey>;

    /// The borrowed version of the key type.
    type BorrowedKey: ?Sized + TryToOwned<Owned = Self::Key>;

    /// Inserts `s` into `self` and returns the intern'd key `Self::Key`.
    fn intern(&mut self, s: &str) -> Result<Self::Key, OutOfMemory>;

    /// Looks up `s` in `self` returning `Some` if it was found or `None` if
    /// it's not present.
    fn lookup<'a>(&'a self, s: &'a str) -> Option<TryCow<'a, Self::BorrowedKey>>;
}

/// For use with [`NameMap`] when no interning should happen and instead string
/// keys are copied as-is.
pub struct NameMapNoIntern;

impl NameMapIntern for NameMapNoIntern {
    type Key = TryString;
    type BorrowedKey = str;

    fn intern(&mut self, s: &str) -> Result<Self::Key, OutOfMemory> {
        TryString::try_from(s)
    }

    fn lookup<'a>(&'a self, s: &'a str) -> Option<TryCow<'a, Self::BorrowedKey>> {
        Some(TryCow::Borrowed(s))
    }
}

impl NameMapIntern for StringPool {
    type Key = Atom;
    type BorrowedKey = Atom;

    fn intern(&mut self, string: &str) -> Result<Atom, OutOfMemory> {
        self.insert(string)
    }

    fn lookup(&self, string: &str) -> Option<TryCow<'_, Atom>> {
        self.get_atom(string).map(TryCow::Owned)
    }
}

/// Returns the canonical name of the semver track that `name` is on.
///
/// This is `name` with its full version, if any, replaced by the canonical
/// version, as defined by [`split_canonical_version`]. Names without a full
/// version are returned as-is. Some examples are:
///
/// * `foo` => `foo`
/// * `foo:bar/baz` => `foo:bar/baz`
/// * `foo:bar/baz@1.1.2` => `foo:bar/baz@1`
/// * `foo:bar/baz@1` => `foo:bar/baz@1`
/// * `foo:bar/baz@0.1.0` => `foo:bar/baz@0.1`
/// * `foo:bar/baz@0.0.1+abc` => `foo:bar/baz@0.0.1`
/// * `foo:bar/baz@0.1.0-rc.2+abc` => `foo:bar/baz@0.1.0-rc.2`
///
/// This is the key that definitions are stored under in a [`NameMap`].
pub fn canonical_name(name: &str) -> &str {
    let Some(at) = name.find('@') else {
        return name;
    };
    match split_canonical_version(&name[at + 1..]) {
        Some((canonical, _suffix)) => &name[..at + 1 + canonical.len()],
        None => name,
    }
}

/// Returns the full version of `name`, if it has one.
fn full_version(name: &str) -> Option<Version> {
    let (_, version) = name.split_once('@')?;
    Version::parse(version).ok()
}

/// Validates that `name` is a valid component import or export name.
///
/// In addition to the checks of [`ComponentName`], this requires that the
/// version of an interface name, if any, is either a full version, such as
/// `a:b/c@0.2.1`, or a canonical version, such as `a:b/c@0.2`. Validation of
/// interface versions is otherwise deferred by [`ComponentName`] since the
/// full version may depend on a `versionsuffix`.
fn validate_name(name: &str) -> Result<()> {
    let parsed = match ComponentName::new_with_features(name, 0, WasmFeatures::all()) {
        Ok(parsed) => parsed,
        Err(e) => bail!("invalid name `{name}`: {}", e.message()),
    };
    if let ComponentNameKind::Interface(interface) = parsed.kind()
        && interface.version(None).is_err()
        && !name
            .split_once('@')
            .is_some_and(|(_, version)| is_canonical_version(version))
    {
        bail!("invalid name `{name}`: version is neither a full nor canonical version");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{NameMap, NameMapNoIntern};
    use crate::prelude::*;

    fn keys<V>(map: &NameMap<TryString, V>) -> Vec<&str> {
        map.raw_iter().map(|(k, _)| &**k).collect()
    }

    #[test]
    fn name_map_smoke() {
        let mut map = NameMap::default();
        let mut intern = NameMapNoIntern;

        map.insert("a", &mut intern, false, 0).unwrap();
        map.insert("b", &mut intern, false, 1).unwrap();

        assert!(map.insert("a", &mut intern, false, 0).is_err());
        assert!(map.insert("a", &mut intern, true, 0).is_ok());

        assert_eq!(map.get("a", &intern), Some(&0));
        assert_eq!(map.get("b", &intern), Some(&1));
        assert_eq!(map.get("c", &intern), None);

        // There's one definition per semver track.
        map.insert("a:b/c@1.0.0", &mut intern, false, 2).unwrap();
        let err = map
            .insert("a:b/c@1.0.1", &mut intern, false, 3)
            .unwrap_err();
        assert!(err.to_string().contains("same semver track"), "{err}");
        assert_eq!(map.get("a:b/c@1.0.0", &intern), Some(&2));
        assert_eq!(map.get("a:b/c@1.0.1", &intern), Some(&2));
        assert_eq!(map.get("a:b/c@1.1.0+a", &intern), Some(&2));
        assert_eq!(map.get("a:b/c@1", &intern), Some(&2));
        assert_eq!(map.get("a:b/c@2", &intern), None);
        assert_eq!(map.get("a:b/c", &intern), None);
        assert_eq!(map.get("a:b/c@1.0", &intern), None);

        // Shadowing replaces the definition on the track.
        map.insert("a:b/c@1.0.1", &mut intern, true, 3).unwrap();
        assert_eq!(map.get("a:b/c@1.0.0", &intern), Some(&3));
        assert_eq!(keys(&map), ["a", "b", "a:b/c@1.0.1"]);

        // Invalid name insertions are rejected.
        map.insert("foo_bar", &mut intern, false, 4).unwrap_err();
        map.insert("a:b/c@1.2", &mut intern, false, 4).unwrap_err();
    }

    #[test]
    fn name_map_get_or_insert_with() {
        let mut map = NameMap::default();
        let mut intern = NameMapNoIntern;

        // Values of at least 10 are mergeable, and merging adds one.
        fn get_or_insert(map: &mut NameMap<TryString, u32>, name: &str, default: u32) -> u32 {
            let v = map
                .get_or_insert_with(name, &mut NameMapNoIntern, false, |v| *v >= 10, || default)
                .unwrap();
            if *v >= 10 {
                *v += 1;
            }
            *v
        }

        assert_eq!(get_or_insert(&mut map, "a:b/c@0.2.1", 10), 11);

        // Any name on the track reopens the definition, which keeps its name.
        assert_eq!(get_or_insert(&mut map, "a:b/c@0.2.1", 0), 12);
        assert_eq!(get_or_insert(&mut map, "a:b/c@0.2.0", 0), 13);
        assert_eq!(get_or_insert(&mut map, "a:b/c@0.2", 0), 14);
        assert_eq!(get_or_insert(&mut map, "a:b/c@0.2.3", 0), 15);
        assert_eq!(keys(&map), ["a:b/c@0.2.1"]);

        // Other tracks get their own definition.
        assert_eq!(get_or_insert(&mut map, "a:b/c@0.3.0", 20), 21);
        assert_eq!(get_or_insert(&mut map, "a:b/c@1", 30), 31);
        assert_eq!(get_or_insert(&mut map, "a:b/c@1.0.1", 0), 32);
        assert_eq!(keys(&map), ["a:b/c@0.2.1", "a:b/c@0.3.0", "a:b/c@1"]);

        // Definitions that can't be merged are an error without shadowing,
        // and are replaced with shadowing.
        map.insert("a:b/d@1.0.0", &mut intern, false, 0).unwrap();
        assert!(
            map.get_or_insert_with("a:b/d@1.0.1", &mut intern, false, |_| false, || 1)
                .is_err()
        );
        let v = map
            .get_or_insert_with("a:b/d@1.0.1", &mut intern, true, |_| false, || 1)
            .unwrap();
        assert_eq!(*v, 1);
        assert_eq!(keys(&map)[3..], ["a:b/d@1.0.1"]);

        // Invalid names are rejected.
        assert!(
            map.get_or_insert_with("a:b/c@1.2", &mut intern, false, |_| true, || 0)
                .is_err()
        );
    }
}
