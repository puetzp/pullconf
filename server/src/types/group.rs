use crate::types::resources::deserialize::Resource;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    #[serde(default)]
    pub resources: Vec<Resource>,
}
