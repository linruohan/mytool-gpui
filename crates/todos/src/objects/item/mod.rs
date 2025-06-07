mod imp;
use std::ops::Deref;

use crate::objects::BaseObject;
pub(crate) use imp::Item;
pub struct ItemLogic {
    pub item: Item,
    pub base: BaseObject,
}
impl Deref for ItemLogic {
    type Target = Item;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl ItemLogic {}
