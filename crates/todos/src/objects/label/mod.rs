mod imp;
use std::ops::Deref;

use crate::objects::BaseObject;
pub(crate) use imp::Label;
pub struct LabelLogic {
    pub label: Label,
    pub base: BaseObject,
}
impl Deref for LabelLogic {
    type Target = Label;

    fn deref(&self) -> &Self::Target {
        &self.label
    }
}

impl LabelLogic {}
