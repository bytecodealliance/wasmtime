//! Functions that perform path lookup manually, one component
//! at a time, with manual symlink resolution.

mod canonical_path;
mod cow_component;
mod open;
mod read_link_one;

use canonical_path::CanonicalPath;
use cow_component::CowComponent;
use read_link_one::read_link_one;

pub(crate) use open::{open, stat};
