use crate::slugify::Slugify;
use crate::BookRecord;
use crate::DirTreeLevel;
use anyhow::Result;
use std::collections::HashMap;

/// These are the only known tags for making into subdirectories. Series is optional and will be
/// omitted if it's not present (the books will be dropped directly under the parent). If you put
/// series in any place but the last before the actual title, weird shit will happen. So don't.
/// build_tree will warn you if you try it.
#[allow(unused)]
pub enum SplitKey {
    Genre,
    Title,
    Author,
    Series,
}


impl SplitKey {
    /// Split one level of leaf book records into groups based on the key (self). Each book's data
    /// is used to determine what it's take for the current SplitKey is, then they get inserted into
    /// a DireTreeLevel::Level HashMap accordingly.
    pub fn split_dir_tree(&self, tree: DirTreeLevel) -> Result<DirTreeLevel> {
        let DirTreeLevel::Leaves(mut leaves) = tree else {
            anyhow::bail!("Can't split a non-leaf DirTreeLevel");
        };
        let mut new_level = HashMap::new();
        for leaf in leaves.drain(..) {
            let tag = self.get_tag(&leaf);
            let split = 
                new_level.entry(tag)
                .or_insert_with(|| DirTreeLevel::Leaves(Vec::new()));
            split.push(leaf)?;
        }
        Ok(DirTreeLevel::Level(new_level))
    }
   
    /// Turn a book record into the tag it wants to be in the filesystem. I am opinionated that
    /// skewer-case is the only true case for directory names. If you don't like it, fork your own
    /// version.
    pub fn get_tag(&self, book:&BookRecord)->String {
        match *self {
            SplitKey::Genre => book.genre.as_ref()
                .map(|segments| segments.join(":"))
                .unwrap_or_else(|| "Unknown".to_owned()).slugify(),
            SplitKey::Title => book.title.as_ref().cloned().unwrap_or_else(||"Unknown".to_owned()).slugify(),
            SplitKey::Author => book.author.as_ref().cloned().unwrap_or_else(||"Unknown".to_owned()).slugify(),
            SplitKey::Series => book.series.as_ref().cloned().unwrap_or_else(||String::new()).slugify(),
        }
    }
}
