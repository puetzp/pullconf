use crate::{Ensure, ResourceMetadata, TriggerMetadata};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Parameters {
    pub ensure: Ensure,
    pub name: String,
    pub command: Vec<String>,
    pub unless: Vec<String>,
    pub environment: Vec<Environment>,
    pub passive: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Environment {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Relationships {
    pub requires: Vec<ResourceMetadata>,
    pub triggers: Vec<TriggerMetadata>,
}
