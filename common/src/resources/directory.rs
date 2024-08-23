use super::group::Name as Groupname;
use super::user::Name as Username;
use crate::{Ensure, ResourceMetadata, SafePathBuf};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Parameters {
    pub path: SafePathBuf,
    pub ensure: Ensure,
    pub owner: Username,
    pub group: Option<Groupname>,
    pub purge: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Relationships {
    pub requires: Vec<ResourceMetadata>,
    pub children: Vec<ChildNode>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ChildNode {
    AptPreference { path: PathBuf },
    Directory { path: SafePathBuf },
    File { path: SafePathBuf },
    Symlink { path: SafePathBuf },
}

impl ChildNode {
    pub fn is_dir(&self, _path: &PathBuf) -> bool {
        matches!(self, Self::Directory { path } if **path == *_path)
    }

    pub fn is_file(&self, _path: &PathBuf) -> bool {
        match self {
            Self::AptPreference { path } => path == path,
            Self::File { path } => **path == *_path,
            _ => false,
        }
    }

    pub fn is_symlink(&self, _path: &PathBuf) -> bool {
        matches!(self, Self::Symlink { path } if **path == *_path)
    }
}
