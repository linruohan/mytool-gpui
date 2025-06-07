mod imp;
use std::ops::Deref;

use crate::objects::BaseObject;
pub(crate) use imp::Section;
pub struct SectionLogic {
    pub section: Section,
    pub base: BaseObject,
}
impl Deref for SectionLogic {
    type Target = Section;

    fn deref(&self) -> &Self::Target {
        &self.section
    }
}

impl SectionLogic {}
