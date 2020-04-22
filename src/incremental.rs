use crate::db::GleamDatabase;
use crate::db::{Analyzed, CodeGen, Sources};
use crate::error::{Error, FileIOAction, FileKind};
use crate::file::write_file;
use crate::project::{
    create_input_from_source, create_module_name, is_gleam_path, SourceCollection,
};
use notify::{watcher, RecursiveMode, Watcher};
use std::error::Error as StdError;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;

pub fn watch_for_changes(
    source_collection: SourceCollection,
    db: &mut GleamDatabase,
) -> Result<(), Error> {
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();

    for dir in source_collection.dirs().into_iter() {
        println!("Will watch {}", dir.to_str().unwrap());
        // Some of these directories may not exist, it's ok to just keep moving
        watcher
            .watch(dir, RecursiveMode::Recursive)
            .or::<()>(Ok(()))
            .unwrap();
    }
    loop {
        match rx.recv() {
            Ok(event) => match event {
                notify::DebouncedEvent::Write(path) => handle_write(path, db, &source_collection),
                notify::DebouncedEvent::Remove(path) => handle_remove(path, db, &source_collection),
                notify::DebouncedEvent::Create(path) => handle_create(path, db, &source_collection),
                notify::DebouncedEvent::Rename(old_path, new_path) => {
                    handle_rename(old_path, new_path, db, &source_collection)
                }
                notify::DebouncedEvent::Chmod(_) => {}
                notify::DebouncedEvent::Error(_, _) => {}
                notify::DebouncedEvent::NoticeRemove(_) => {}
                notify::DebouncedEvent::NoticeWrite(_) => {}
                notify::DebouncedEvent::Rescan => {}
            },
            Err(e) => {
                println!("watch error: {:?}", e);
                return Err(Error::FileIO {
                    action: FileIOAction::Read,
                    kind: FileKind::File,
                    err: Some(e.description().to_string()),
                    path: PathBuf::from("."),
                });
            }
        }
    }
}

fn handle_write(path: PathBuf, db: &mut GleamDatabase, source_collection: &SourceCollection) {
    match std::fs::read_to_string(path.clone()).map_err(|err| Error::FileIO {
        action: FileIOAction::Read,
        kind: FileKind::File,
        path: path.to_path_buf(),
        err: Some(err.to_string()),
    }) {
        Ok(src) => match create_input_from_source(path.clone(), src, source_collection) {
            Some(input) => {
                if is_gleam_path(path, input.source_base_path.clone()) {
                    let name = create_module_name(&input);
                    println!("{} changed. Recompiling...", name.clone());
                    db.set_source_file(name.clone(), input);
                    match db.invalidate_module(name) {
                        Ok(new_files) => {
                            new_files
                                .into_iter()
                                .for_each(|file| match write_file(file) {
                                    Err(e) => e.pretty_print(),
                                    _ => (),
                                });
                            println!("Done.");
                        }
                        Err(err) => {
                            err.pretty_print();
                        }
                    };
                }
            }
            None => (),
        },
        Err(err) => err.pretty_print(),
    }
}

fn handle_remove(path: PathBuf, db: &mut GleamDatabase, source_collection: &SourceCollection) {
    let mut all_sources = db.sources(());
    match create_input_from_source(path.clone(), "".to_string(), &source_collection) {
        Some(input) => {
            if is_gleam_path(path.clone(), input.source_base_path.clone()) {
                let name = create_module_name(&input);
                all_sources.remove(&name);
                db.set_sources((), all_sources);
                match db.check_dependents(name) {
                    Ok(_) => {}
                    Err(e) => e.pretty_print(),
                }
                match std::fs::remove_file(input.path.clone()).map_err(|e| Error::FileIO {
                    action: FileIOAction::Delete,
                    kind: FileKind::File,
                    path: input.path,
                    err: Some(e.to_string()),
                }) {
                    Err(err) => err.pretty_print(),
                    _ => (),
                }
            }
        }
        None => {}
    }
}

fn handle_create(path: PathBuf, db: &mut GleamDatabase, source_collection: &SourceCollection) {
    match std::fs::read_to_string(path.clone()).map_err(|err| Error::FileIO {
        action: FileIOAction::Read,
        kind: FileKind::File,
        path: path.to_path_buf(),
        err: Some(err.to_string()),
    }) {
        Ok(src) => {
            let mut all_sources = db.sources(());
            match create_input_from_source(path.clone(), src, &source_collection) {
                Some(input) => {
                    if is_gleam_path(path, input.source_base_path.clone()) {
                        let name = create_module_name(&input);
                        all_sources.insert(name.clone());
                        db.set_sources((), all_sources);
                        db.set_source_file(name.clone(), input);
                        match db.generate_new_module(name.clone()) {
                            Ok(files) => {
                                files.into_iter().for_each(|file| match write_file(file) {
                                    Err(e) => e.pretty_print(),
                                    _ => (),
                                })
                            }
                            Err(e) => e.pretty_print(),
                        }
                    }
                }
                None => {}
            }
        }
        Err(e) => e.pretty_print(),
    }
}

fn handle_rename(
    old_path: PathBuf,
    new_path: PathBuf,
    db: &mut GleamDatabase,
    source_collection: &SourceCollection,
) {
    let inputs = (
        create_input_from_source(old_path, "".to_string(), &source_collection),
        create_input_from_source(new_path, "".to_string(), &source_collection),
    );
    match inputs {
        (Some(old), Some(current)) => {
            let old_name = create_module_name(&old);
            let current_name = create_module_name(&current);
            let old_src = db.source_file(old_name.clone());
            let mut all_sources = db.sources(());
            all_sources.remove(&old_name);
            all_sources.insert(current_name.clone());
            db.set_source_file(current_name.clone(), old_src);
            match db.check_dependents(current_name) {
                Ok(_) => {}
                Err(e) => e.pretty_print(),
            }
        }
        _ => {}
    }
}
