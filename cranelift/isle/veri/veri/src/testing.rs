use std::{cmp::Ordering, fmt::Debug};

pub fn assert_strictly_increasing<T>(elements: &[T])
where
    T: PartialOrd + Debug,
{
    elements.windows(2).for_each(|p| assert!(p[0] < p[1]));
}

pub fn assert_partial_order_properties<T>(elements: &[T])
where
    T: PartialOrd + Debug,
{
    // Equality
    for a in elements {
        for b in elements {
            assert_eq!(
                a == b,
                a.partial_cmp(b) == Some(Ordering::Equal),
                "equality property failed: a={a:?} b={b:?}"
            );
        }
    }

    // Transitivity
    for a in elements {
        for b in elements {
            for c in elements {
                assert!(
                    !(a < b && b < c && !(a < c)),
                    "transitivity property failed: a={a:?} b={b:?} c={c:?}"
                );
            }
        }
    }

    // Duality
    for a in elements {
        for b in elements {
            assert_eq!(a < b, b > a, "duality property failed: a={a:?} b={b:?}");
        }
    }
}
