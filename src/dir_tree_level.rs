use crate::SplitKey;
use anyhow::Result;
use std::path::Path;
use std::collections::HashMap;
use crate::BookRecord;


pub enum DirTreeLevel {
    Leaves(Vec<BookRecord>),
    Level(HashMap<String,DirTreeLevel>)
}

impl DirTreeLevel {
    /// Take the split tree structure and flatten it into a vec of BookRecords that have their
    /// destination members filled correctly. These are fed into a move/symlink task.
    pub fn compile_move_ops(self, srcdir: &Path, outdir: &Path) -> Result<Vec<BookRecord>> {
        let mut local_result = Vec::new();
        match self {
            DirTreeLevel::Level(level) => {
                for (subdir,inner_level) in level.into_iter() {
                    // Sometimes, there is no tag for a level. In that case, just don't make a new
                    // directory, instead dumping the books directly into the current level. 
                    let new_dir = if !subdir.is_empty(){
                        outdir.join(subdir)
                    } else {
                        outdir.to_owned()
                    };
                    local_result.extend(inner_level.compile_move_ops(srcdir,&new_dir)?.drain(..));
                }
            }
            DirTreeLevel::Leaves(mut leaves) => {
                for mut book in leaves.drain(..) {
                    let ext = book.resolve_extension(&srcdir)?;
                    book.location = srcdir.join(&book.location).with_added_extension(ext);
                    book.destination = Some(outdir
                        .join(&book.location.file_name()
                            .ok_or_else(||anyhow::anyhow!("book path doesn't have filename: {:?}", book.location))?));
                    local_result.push(book);
                }
            }       
        }
        Ok(local_result)
    }

    /// Convenience function to not have to pattern deconstruct when I want to add stuff to a
    /// HashMap::Entry
    pub fn push(&mut self, book: BookRecord) -> Result<()> {
        match self {
            DirTreeLevel::Leaves(leaves) => leaves.push(book),
            _ => anyhow::bail!("Can't push book recored into non-leaf DirTreeLevel")
        }
        Ok(())
    }

    /// Take a sequence of SplitKeys and apply them recursively to the current Leaves. If this level
    /// isn't Leaves, it bails out.
    pub fn build_tree(self, keys: &[SplitKey]) -> Result<Self> {
        if keys.len() < 1 {
            anyhow::bail!("Can't build_tree with empty keys[]");
        }
        let key = &keys[0];
        let remaining_keys = &keys[1..];
        let DirTreeLevel::Level(split) = key.split_dir_tree(self)? else {
            unreachable!("split_dir_tree always returns Level")
        };
        if remaining_keys.is_empty() {
            Ok(DirTreeLevel::Level(split))
        } else {
            Ok(DirTreeLevel::Level(split.into_iter().map(|(key,leaf)|{
                Ok((key, leaf.build_tree(remaining_keys)?))
            }).collect::<Result<_>>()?))
        }
    }
}


