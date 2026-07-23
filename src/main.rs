#![feature(try_blocks)]

use serde::Deserialize;
use clap::Parser;
use std::path::{PathBuf,Path};
use anyhow::{Result,self};
use std::collections::HashMap;
use slugify::slugify;

pub trait Slugify:AsRef<str> {
    fn slugify(&self)->String {
        slugify::slugify!(self.as_ref()).replace("-s-", "s-")
    }
}

impl Slugify for String{}

pub mod splitter {
    pub fn split_genre_string<'de,D>(deserializer:D) -> Result<Option<Vec<String>>,D::Error> 
    where D: serde::de::Deserializer<'de> {
        let st:&str = serde::de::Deserialize::deserialize(deserializer)?;
        Ok(Some(st.split(':').map(|s| s.trim().to_owned()).collect()))
    }
}

#[derive(Deserialize,Debug)]
pub struct BookRecord {
    #[serde(default)]
    pub destination: Option<PathBuf>,
    #[serde(rename="filename")]
    pub location: PathBuf,

    #[serde(default)] pub title: Option<String>,
    #[serde(default,deserialize_with="self::splitter::split_genre_string")] pub genre: Option<Vec<String>>,
    #[serde(default,rename="series_name")] pub series: Option<String>,
    #[serde(default)] pub author: Option<String>,
    #[serde(default)] pub sequence: usize,
}

pub enum DirTreeLevel {
    Leaves(Vec<BookRecord>),
    Level(HashMap<String,DirTreeLevel>)
}

impl DirTreeLevel {
    /// Take the split tree structure and flatten it into a vec of BookRecords that have their
    /// destination members filled correctly. These are fed into a move/symlink task.
    pub fn compile_move_ops(self, cwd:&Path) -> Result<Vec<BookRecord>> {
        let mut local_result = Vec::new();
        match self {
            DirTreeLevel::Level(level) => {
                for (subdir,inner_level) in level.into_iter() {
                    let new_dir = if !subdir.is_empty(){
                        cwd.join(subdir)
                    } else {
                        cwd.to_owned()
                    };
                    local_result.extend(inner_level.compile_move_ops(&new_dir)?.drain(..));
                }
            }
            DirTreeLevel::Leaves(mut leaves) => {
                for mut book in leaves.drain(..) {
                    book.destination = Some(cwd.to_owned());
                    local_result.push(book);
                }
            }       
        }
        Ok(local_result)
    }

    pub fn push(&mut self, book: BookRecord) -> Result<()> {
        match self {
            DirTreeLevel::Leaves(leaves) => leaves.push(book),
            _ => anyhow::bail!("Can't push book recored into non-leaf DirTreeLevel")
        }
        Ok(())
    }

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

pub enum SplitKey {
    Genre,
    Title,
    Author,
    Series,
}

impl SplitKey {
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
    pub books_path: String,
}

#[derive(Deserialize)]
pub struct BooksVec(Vec<BookRecord>);

fn main(){
    use SplitKey::*;
    // 1. read books.json into DirTreeLevel::Leaves
    // 2. build_tree
    // 3. compile_move_ops
    let args = Args::parse();
    let res = try {
        let book_files_dir = PathBuf::from(&args.books_path).parent()
            .ok_or_else(|| anyhow::anyhow!("books_path has no parent dir"))?
            .canonicalize().map_err(|e| anyhow::anyhow!("Can't canonicalize book path dir: {e}"))?
            .join("books");
        let all_books:BooksVec = serde_json::from_str(
            &std::fs::read_to_string(&args.books_path)
                .map_err(|e| anyhow::Error::from(e))?)
            .map_err(|e| anyhow::Error::from(e))?;
        let root = DirTreeLevel::Leaves(all_books.0);
        let tree = root.build_tree(&[Genre,Author,Series,Title])?;
        let ops = tree.compile_move_ops(&book_files_dir)?;
        for op in ops.iter() {
            println!("Move {:?}\n  -> {:?}",
                op.location, 
                op.destination.as_ref()
                    .ok_or_else(||anyhow::anyhow!("No destination path for {:#?}", op))?);
        }
    };
    if let Err(e) = res {
        eprintln!("Error: {e}");
        std::process::exit(1)
    }
}

