use crate::{
    common::{DesktopEntry, ExecMode},
    Error, Result,
};
use std::{convert::TryFrom, ffi::OsString, fmt::Display, path::PathBuf, str::FromStr};

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Handler(pub(crate) OsString);

impl Display for Handler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string_lossy())
    }
}

impl FromStr for Handler {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::resolve(s.into())
    }
}

impl Handler {
    pub(crate) fn assume_valid(name: OsString) -> Self {
        Self(name)
    }

    pub(crate) fn get_path(name: &std::ffi::OsStr) -> Option<PathBuf> {
        let mut path = PathBuf::from("applications");
        path.push(name);
        xdg::BaseDirectories::new().ok()?.find_data_file(path)
    }

    pub(crate) fn resolve(name: OsString) -> Result<Self> {
        let path =
            Self::get_path(&name).ok_or_else(|| Error::NotFound(name.to_string_lossy().into()))?;
        DesktopEntry::try_from(path)?;
        Ok(Self(name))
    }

    pub(crate) fn get_entry(&self) -> Result<DesktopEntry> {
        DesktopEntry::try_from(Self::get_path(&self.0).unwrap())
    }

    pub(crate) fn launch(&self, args: Vec<String>) -> Result<()> {
        self.get_entry()?.exec(ExecMode::Launch, args)
    }

    pub(crate) fn open(&self, args: Vec<String>) -> Result<()> {
        self.get_entry()?.exec(ExecMode::Open, args)
    }
}
