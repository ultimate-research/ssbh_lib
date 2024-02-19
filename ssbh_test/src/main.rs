use clap::Parser;
use rayon::prelude::*;
use std::{io::Cursor, path::Path};

/// Test read/write for all files recursively in a game dump.
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// The root folder of the game dump
    root_folder: String,
}

fn main() {
    let cli = Cli::parse();

    let folder = Path::new(&cli.root_folder);
    let start = std::time::Instant::now();

    let patterns = ["*.{numatb,numdlb,numshb,nusktb,nurpdb,nufxlb,nuanmb,nuhlpb,nushdb}"];
    globwalk::GlobWalkerBuilder::from_patterns(folder, &patterns)
        .build()
        .unwrap()
        .filter_map(|p| p.ok())
        .par_bridge()
        .for_each(|path| {
            check_read_write_ssbh(path.path());
        });

    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.numshexb"])
        .build()
        .unwrap()
        .filter_map(|p| p.ok())
        .par_bridge()
        .for_each(|path| {
            check_read_write_meshex(path.path());
        });

    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.adjb"])
        .build()
        .unwrap()
        .filter_map(|p| p.ok())
        .par_bridge()
        .for_each(|path| {
            check_read_write_adj(path.path());
        });

    println!("Finished in {:?}", start.elapsed());
}

fn check_read_write_ssbh(path: &Path) {
    let before = std::fs::read(path).unwrap();
    match ssbh_lib::SsbhFile::read(&mut Cursor::new(&before)) {
        Ok(ssbh) => {
            // Check any supported file for 1:1 read/write.
            let mut writer = Cursor::new(Vec::new());
            ssbh.write(&mut writer).unwrap();
            if before != writer.into_inner() {
                println!("Read/write not 1:1 for {path:?}");
            }
        }
        _ => {
            println!("Error reading {path:?}");
        }
    }
}

fn check_read_write_meshex(path: &Path) {
    let before = std::fs::read(path).unwrap();
    match ssbh_lib::formats::meshex::MeshEx::read(&mut Cursor::new(&before)) {
        Ok(data) => {
            // Check any supported file for 1:1 read/write.
            let mut writer = Cursor::new(Vec::new());
            data.write(&mut writer).unwrap();
            if before != writer.into_inner() {
                println!("Read/write not 1:1 for {path:?}");
            }
        }
        _ => {
            println!("Error reading {path:?}");
        }
    }
}

fn check_read_write_adj(path: &Path) {
    let before = std::fs::read(path).unwrap();
    match ssbh_lib::formats::adj::Adj::read(&mut Cursor::new(&before)) {
        Ok(data) => {
            // Check any supported file for 1:1 read/write.
            let mut writer = Cursor::new(Vec::new());
            data.write(&mut writer).unwrap();
            if before != writer.into_inner() {
                println!("Read/write not 1:1 for {path:?}");
            }
        }
        _ => {
            println!("Error reading {path:?}");
        }
    }
}
