pub mod imp;
use std::ops::Deref;

use crate::{Store, objects::BaseObject};
pub(crate) use imp::Project;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLogic {
    pub project: Project,
    pub base: BaseObject,
}
impl Deref for ProjectLogic {
    type Target = Project;

    fn deref(&self) -> &Self::Target {
        &self.project
    }
}

impl Default for ProjectLogic {
    fn default() -> Self {
        let project = Project::default();
        let base = BaseObject::new(
            "Projects".to_string(),
            format!("{};{}", "projects", "filters"),
            "folder-symbolic".to_string(),
            "projects-view".to_string(),
        );
        Self { project, base }
    }
}
