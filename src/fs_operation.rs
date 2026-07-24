use std::path::{Path, PathBuf};
use anyhow::Result;

use crate::book_record::BookRecord;

#[allow(unused)]
pub enum FsOperation {
    MakeDirectory{
        target: PathBuf,
    },
    FileLocation{
        source: PathBuf,
        destination: PathBuf,
    }
}

#[allow(unused)]
pub trait ShellCommand {
    fn as_shell_command(&self) -> Result<String>;
}

#[allow(unused)]
pub trait ApplyOp{
    fn apply(&self) -> Result<()>;
}

fn escape_quotes(s:&str) -> String {
    s.replace("\"", "\\\"")
}

impl ShellCommand for FsOperation {
    fn as_shell_command(&self) -> Result<String> {
        match *self { 
            FsOperation::MakeDirectory{ref target} => {
                Ok(format!("mkdir -p \"{}\"", escape_quotes(&*target.to_string_lossy())))
            },
            FsOperation::FileLocation{ref source,ref destination} => {
                Ok(format!("ln -sf \"{}\" \"{}\"", 
                        escape_quotes(&*source.to_string_lossy()), 
                        escape_quotes(&*destination.to_string_lossy())))
            }
        }
    }
}

impl ApplyOp for FsOperation {
    fn apply(&self) -> Result<()> {
        match *self {
            FsOperation::MakeDirectory{ref target} => {
                eprintln!("create_dir_all({target:?})");
                Ok(std::fs::create_dir_all(target)?)
            },
            FsOperation::FileLocation{ref source, ref destination} => {
                eprintln!("fs::symlink({source:?}, {destination:?})");
                Ok(std::os::unix::fs::symlink(source,destination)?)
            }
        }
    }
}

impl From<&Path> for FsOperation {
    fn from(path: &Path) -> Self {
        FsOperation::MakeDirectory { target: path.to_owned() }
    }
}

impl TryFrom<&BookRecord> for FsOperation {
    type Error = anyhow::Error;
    fn try_from(book: &BookRecord) -> Result<FsOperation> {
        Ok(FsOperation::FileLocation{
            source: book.location.clone(),
            destination: book.destination
                .as_ref().cloned()
                .ok_or_else(|| anyhow::anyhow!("Book has no destination set"))?,
        })
    }
}
