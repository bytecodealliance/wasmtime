use super::Entry;
use crate::handle::{Advice, Filesize, HandleRights, Rights};
use crate::Result;

impl Entry {
    pub fn fd_advise(&self, offset: Filesize, len: Filesize, advice: Advice) -> Result<()> {
        let required_rights = HandleRights::from_base(Rights::FD_ADVISE);
        self.as_handle(&required_rights)?
            .advise(advice, offset, len)
    }
}
