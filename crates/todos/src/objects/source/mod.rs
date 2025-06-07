mod imp;
use std::ops::Deref;

use crate::objects::BaseObject;
pub(crate) use imp::Source;
pub struct SourceLogic {
    pub source: Source,
    pub base: BaseObject,
}
impl Deref for SourceLogic {
    type Target = Source;

    fn deref(&self) -> &Self::Target {
        &self.source
    }
}

impl SourceLogic {}
