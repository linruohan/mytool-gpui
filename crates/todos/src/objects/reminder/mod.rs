mod imp;
use std::ops::Deref;

use crate::objects::BaseObject;
pub(crate) use imp::Reminder;
pub struct ReminderLogic {
    pub reminder: Reminder,
    pub base: BaseObject,
}
impl Deref for ReminderLogic {
    type Target = Reminder;

    fn deref(&self) -> &Self::Target {
        &self.reminder
    }
}

impl ReminderLogic {}
