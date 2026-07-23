#![feature(try_blocks)]

use serde::Deserialize;
use clap::Parser;
use std::{collections::HashSet, path::{Path, PathBuf}};
use anyhow::{Result,self};
use std::collections::HashMap;
use slugify::slugify;

pub trait Slugify:AsRef<str> {
    fn slugify(&self)->String {
        slugify::slugify!(self.as_ref()).replace("-s-", "s-")
    }
}

impl Slugify for String{}

pub fn split_genre_string<'de,D>(deserializer:D) -> Result<Option<Vec<String>>,D::Error> 
where D: serde::de::Deserializer<'de> {
    let st:&str = serde::de::Deserialize::deserialize(deserializer)?;
    Ok(Some(st.split(':').map(|s| s.trim().to_owned()).collect()))
}

#[derive(Deserialize,Debug)]
pub struct BookRecord {
    #[serde(default)]
    pub destination: Option<PathBuf>,
    #[serde(rename="filename")]
    pub location: PathBuf,

    #[serde(default)] pub title: Option<String>,
    #[serde(default,deserialize_with="self::split_genre_string")] pub genre: Option<Vec<String>>,
    #[serde(default,rename="series_name")] pub series: Option<String>,
    #[serde(default)] pub author: Option<String>,
    #[serde(default)] pub sequence: usize,
}

impl BookRecord {
    pub fn resolve_extension(&self, dir: &Path) -> Result<&'static str> {
        let extensions = ["m4b","mp3","aax"];
        for ext in extensions {
            if std::fs::exists(dir.join(self.location.with_added_extension(ext)))? {
                return Ok(ext)
            }
        }
        Err(anyhow::anyhow!("No extension found for book: {:?}. Dir was {:?}",self.title, dir))
    }    
}

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

/// These are the only known tags for making into subdirectories. Series is optional and will be
/// omitted if it's not present (the books will be dropped directly under the parent). If you put
/// series in any place but the last before the actual title, weird shit will happen. So don't.
/// build_tree will warn you if you try it.
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

#[derive(clap::Parser)]
pub struct Args{
    /// location of the books.json file in your OpenAudible downloads target
    pub books_path: String,
    /// Location of the target folder where the directory tree should be built
    #[arg(short,long)]
    pub output_path: String,
}

#[derive(Deserialize)]
pub struct BooksVec(Vec<BookRecord>);

fn main(){
    use SplitKey::*;
    // 1. read books.json into DirTreeLevel::Leaves
    // 2. build_tree
    // 3. compile_move_ops
    let args = Args::parse();
    let res:anyhow::Result<()> = try {
        eprintln!("Starting");
        let book_files_dir = PathBuf::from(&args.books_path).parent()
            .ok_or_else(|| anyhow::anyhow!("books_path has no parent dir"))?
            .canonicalize().map_err(|e| anyhow::anyhow!("Can't canonicalize book path dir: {e}"))?
            .join("books");
        let mut output_path = PathBuf::from(args.output_path);
        if !std::fs::exists(&output_path).map_err(anyhow::Error::from)? {
            let parent = output_path.parent()
                .ok_or_else(||anyhow::anyhow!("output path must have parent dir"))?
                .canonicalize()
                .map_err(|e| anyhow::anyhow!("Parent of output directory must exist before running this tool! {e}"))?;
            output_path = parent.join(output_path.iter().last().unwrap());
        }
        let all_books:BooksVec = serde_json::from_str(
            &std::fs::read_to_string(&args.books_path)
                .map_err(|e| anyhow::Error::from(e))?)
            .map_err(|e| anyhow::Error::from(e))?;
        let root = DirTreeLevel::Leaves(all_books.0);
        let tree = root.build_tree(&[Genre,Author,Series])?;
        let ops = tree.compile_move_ops(&book_files_dir, &output_path)?;
        let mut dir_cache = HashSet::new();
        for op in ops.iter() {
            if let Some(parent) = op.destination.as_ref().and_then(|d|d.parent()) {
                // make sure each directory exists before putting files into it.
                if !dir_cache.contains(parent) {
                    println!("mkdir -p {parent:?}");
                    dir_cache.insert(parent.to_owned());
                }
            }
            println!("ln -sf {:?} {:?}",
                &op.location, 
                op.destination.as_ref()
                .ok_or_else(||anyhow::anyhow!("No destination path for {:#?}", op))?);
        }
    };
    if let Err(e) = res {
        eprintln!("Error: {e}");
        std::process::exit(1)
    }
}

