use arbitrary::{Arbitrary, Unstructured};

/// The control plane of chaos mode.
/// Please see the [crate-level documentation](crate).
#[derive(Debug, Clone, Default)]
pub struct ControlPlane {
    data: Vec<u8>,
}

impl Arbitrary<'_> for ControlPlane {
    fn arbitrary<'a>(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            data: u.arbitrary()?,
        })
    }
}

impl ControlPlane {
    /// Returns a pseudo-random boolean if the control plane was constructed
    /// with `arbitrary`.
    ///
    /// The default value `false` will always be returned if the
    /// pseudo-random data is exhausted or the control plane was constructed
    /// with `default`.
    pub fn get_decision(&mut self) -> bool {
        self.data.pop().unwrap_or_default() & 1 == 1
    }

    /// Shuffles the items in the slice into a pseudo-random permutation if
    /// the control plane was constructed with `arbitrary`.
    ///
    /// The default operation, to leave the slice unchanged, will always be
    /// performed if the pseudo-random data is exhausted or the control
    /// plane was constructed with `default`.
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        let mut u = Unstructured::new(&self.data);

        // adapted from:
        // https://docs.rs/arbitrary/1.3.0/arbitrary/struct.Unstructured.html#examples-1
        let mut to_permute = &mut slice[..];

        while to_permute.len() > 1 {
            if let Ok(idx) = u.choose_index(to_permute.len()) {
                to_permute.swap(0, idx);
                to_permute = &mut to_permute[1..];
            } else {
                break;
            }
        }

        self.data = u.take_rest().to_vec();
    }

    /// Returns a new iterator over the same items as the input iterator in
    /// a pseudo-random order if the control plane was constructed with
    /// `arbitrary`.
    ///
    /// The default value, an iterator with an unchanged order, will always
    /// be returned if the pseudo-random data is exhausted or the control
    /// plane was constructed with `default`.
    pub fn change_order<T>(&mut self, iter: impl Iterator<Item = T>) -> impl Iterator<Item = T> {
        let mut slice: Vec<_> = iter.collect();
        self.shuffle(&mut slice);
        slice.into_iter()
    }
}
