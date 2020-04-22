// TODO
// #![deny(warnings)]

mod ast;
mod db;
mod doc;
mod erl;
mod error;
mod file;
mod format;
mod incremental;
mod new;
mod parser;
mod pretty;
mod project;
mod typ;

lalrpop_mod!(
    #[allow(deprecated)]
    #[allow(clippy::all)]
    #[allow(dead_code)]
    #[allow(unused_parens)]
    grammar
);

#[macro_use]
extern crate im;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

#[macro_use]
extern crate lalrpop_util;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate handlebars;

#[macro_use]
extern crate salsa;

use crate::db::{CodeGen, Sources};
use crate::error::Error;
use crate::project::SourceCollection;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use structopt::clap::AppSettings;
use structopt::StructOpt;
use strum::VariantNames;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(StructOpt, Debug)]
#[structopt(global_settings = &[AppSettings::ColoredHelp, AppSettings::VersionlessSubcommands])]
enum Command {
    #[structopt(name = "build", about = "Compile a project")]
    Build {
        #[structopt(help = "location of the project root", default_value = ".")]
        path: String,
        #[structopt(help = "generate docs for this package as well", long)]
        doc: bool,
        #[structopt(
            help = "after the initial build, watch for chnages to Gleam source",
            long
        )]
        watch: bool,
    },

    #[structopt(name = "new", about = "Create a new project")]
    New {
        #[structopt(help = "name of the project")]
        name: String,

        #[structopt(long = "description", help = "description of the project")]
        description: Option<String>,

        #[structopt(help = "location of the project root")]
        path: Option<String>,

        #[structopt(
            long = "template",
            possible_values = &new::Template::VARIANTS,
            case_insensitive = true,
            default_value = "lib"
        )]
        template: new::Template,
    },

    #[structopt(name = "format", about = "Format source code")]
    Format {
        #[structopt(help = "files to format", required_unless = "stdin")]
        files: Vec<String>,

        #[structopt(
            help = "read source from standard in",
            long = "stdin",
            conflicts_with = "files"
        )]
        stdin: bool,

        #[structopt(
            help = "check if inputs are formatted without changing them",
            long = "check"
        )]
        check: bool,
    },
}

#[derive(Deserialize)]
struct ProjectConfig {
    name: String,
}

fn main() {
    let result = match Command::from_args() {
        Command::Build { path, doc, watch } => command_build(path, doc, watch),

        Command::Format {
            stdin,
            files,
            check,
        } => crate::format::command::run(stdin, check, files),

        Command::New {
            name,
            description,
            path,
            template,
        } => crate::new::create(template, name, description, path, VERSION),
    };

    if let Err(e) = result {
        e.pretty_print();
        std::process::exit(1);
    }
}

fn command_build(root: String, write_docs: bool, watch: bool) -> Result<(), Error> {
    // Read gleam.toml
    let project_config = read_project_config(&root)?;

    let root_path = PathBuf::from(&root).canonicalize().unwrap();
    let lib_dir = root_path.join("_build").join("default").join("lib");
    let checkouts_dir = root_path.join("_checkouts");

    let source_collection = SourceCollection::new(
        root_path.join("src"),
        root_path.join("test"),
        vec![lib_dir, checkouts_dir],
    );
    let srcs = source_collection.sources(&project_config.name)?;

    let mut db = db::GleamDatabase::default();

    db.set_sources((), HashSet::with_capacity(srcs.len()));

    println!("Compiling {} sources", srcs.len());
    for src in srcs.into_iter() {
        let name = crate::project::create_module_name(&src);
        db.set_source_file(name.clone(), src);
        let mut sources = db.sources(());
        sources.insert(name);
        db.set_sources((), sources);
    }

    // Generate outputs (Erlang code, html documentation, etc)
    let output_files = db.generate_project_code()?;
    if write_docs {
        todo!()
    }

    // Delete the gen directory before generating the newly compiled files
    for dir in ["gen", "doc"].iter() {
        let dir = root_path.join(dir);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|e| Error::FileIO {
                action: error::FileIOAction::Delete,
                kind: error::FileKind::Directory,
                path: dir,
                err: Some(e.to_string()),
            })?;
        }
    }
    for file in output_files {
        crate::file::write_file(file)?;
    }
    println!("Done!");

    if watch {
        println!("Waiting for changes...");
        crate::incremental::watch_for_changes(source_collection, &mut db)
    } else {
        Ok(())
    }
}

fn read_project_config(root: &str) -> Result<ProjectConfig, Error> {
    let config_path = PathBuf::from(root).join("gleam.toml");

    let mut file = File::open(&config_path).map_err(|e| Error::FileIO {
        action: error::FileIOAction::Open,
        kind: error::FileKind::File,
        path: config_path.clone(),
        err: Some(e.to_string()),
    })?;

    let mut toml = String::new();
    file.read_to_string(&mut toml).map_err(|e| Error::FileIO {
        action: error::FileIOAction::Read,
        kind: error::FileKind::File,
        path: config_path.clone(),
        err: Some(e.to_string()),
    })?;

    let project_config = toml::from_str(&toml).map_err(|e| Error::FileIO {
        action: error::FileIOAction::Parse,
        kind: error::FileKind::File,
        path: config_path.clone(),
        err: Some(e.to_string()),
    })?;

    Ok(project_config)
}
