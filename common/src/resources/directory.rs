use crate::SafePathBuf;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ChildNode {
    Directory { path: SafePathBuf },
    File { path: SafePathBuf },
    Symlink { path: SafePathBuf },
}

impl ChildNode {
    pub fn is_dir(&self, _path: &PathBuf) -> bool {
        matches!(self, Self::Directory { path } if **path == *_path)
    }

    pub fn is_file(&self, _path: &PathBuf) -> bool {
        matches!(self, Self::File { path } if **path == *_path)
    }

    pub fn is_symlink(&self, _path: &PathBuf) -> bool {
        matches!(self, Self::Symlink { path } if **path == *_path)
    }
}
