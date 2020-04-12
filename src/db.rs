use salsa::{Database, Query};
use std::collections::HashSet;

use crate::project::Input;

#[salsa::query_group(SourcesStorage)]
pub trait Sources: Database {
    #[salsa::input]
    fn source_file(&self, file_path: String) -> Input;

    #[salsa::input]
    fn sources(&self, key: ()) -> HashSet<String>;

    fn all_sources(&self) -> Vec<Input>;
}

#[salsa::query_group(ModulesStorage)]
pub trait Modules: Sources {
    fn module_ast(
        &self,
        file_path: String,
    ) -> Result<crate::ast::UntypedModule, crate::error::Error>;

    fn dependencies(&self, file_path: String) -> Vec<String>;
}

pub fn all_sources(db: &impl Sources) -> Vec<Input> {
    let mut sources = db.sources(());
    sources.drain().map(|src| db.source_file(src)).collect()
}

pub fn module_ast(
    db: &impl Modules,
    file_path: String,
) -> Result<crate::ast::UntypedModule, crate::error::Error> {
    let source = db.source_file(file_path);
    let (stripped, _) = crate::parser::strip_extra(&source.src);
    crate::grammar::ModuleParser::new()
        .parse(&stripped)
        .map_err(|e| crate::error::Error::Parse {
            path: source.path.clone(),
            src: source.src.clone(),
            error: e.map_token(|crate::grammar::Token(a, b)| (a, b.to_string())),
        })
}

pub fn dependencies(db: &impl Modules, file_path: String) -> Vec<String> {
    match db.module_ast(file_path) {
        Ok(module) => module
            .statements
            .iter()
            .flat_map(|s| match s {
                crate::ast::Statement::Import { module, .. } => Some(module.join("/")),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

#[salsa::database(SourcesStorage, ModulesStorage)]
#[derive(Default)]
pub struct GleamDatabase {
    runtime: salsa::Runtime<GleamDatabase>,
}

impl Database for GleamDatabase {
    fn salsa_runtime(&self) -> &salsa::Runtime<Self> {
        &self.runtime
    }

    fn salsa_runtime_mut(&mut self) -> &mut salsa::Runtime<Self> {
        &mut self.runtime
    }
}
