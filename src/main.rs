#![feature(try_blocks)]

use serde::Deserialize;
use clap::Parser;
use std::{collections::HashSet, path::PathBuf};
use anyhow::Result;

mod book_record;
use book_record::BookRecord;
mod dir_tree_level;
use dir_tree_level::DirTreeLevel;
mod slugify;
mod split_key;
use split_key::SplitKey;
mod fs_operation;
use fs_operation::FsOperation;

use crate::fs_operation::{ApplyOp, ShellCommand};

#[derive(clap::Parser)]
pub struct Args{
    /// location of the books.json file in your OpenAudible downloads target
    pub books_path: String,
    /// Location of the target folder where the directory tree should be built
    #[arg(short,long)]
    pub output_path: String,

    /// Directly make the changes without outputting a shell script.
    #[arg(short,long,action)]
    pub apply:bool,

    /// Output a shell script to make the changes (default)
    #[arg(short,long,action)]
    pub shell:bool,
}

#[derive(Deserialize)]
pub struct BooksVec(Vec<BookRecord>);

fn run_op(op: &FsOperation, apply: bool, shell: bool) -> Result<()> {
    if shell {
        println!("{}", op.as_shell_command()?);
    }
    if apply {
        op.apply()?;
    }
    Ok(())
}

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
        let tree = root.build_tree(&[Author,Series])?;
        let ops = tree.compile_move_ops(&book_files_dir, &output_path)?;
        let mut dir_cache = HashSet::new();
        for op in ops.iter() {
            let dir = op.target_dir()?;
                // make sure each directory exists before putting files into it.
            if !dir_cache.contains(&dir) {
                let mkdir = FsOperation::from(dir.as_ref());
                run_op(&mkdir,args.apply, args.shell)?;
                dir_cache.insert(dir.to_owned());
            }
            let fop = FsOperation::try_from(op)?;
            run_op(&fop, args.apply, args.shell)?;
        }
    };
    if let Err(e) = res {
        eprintln!("Error: {e}");
        std::process::exit(1)
    }
}

