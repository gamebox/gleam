use super::sources::Sources;
use crate::ast::UntypedModule;
use crate::error::Error;
use crate::ast::Statement;

#[salsa::query_group(ModulesStorage)]
pub trait Modules: Sources {
    fn module_ast(
        &self,
        file_path: String,
    ) -> Result<UntypedModule, Error>;

    fn dependencies(&self, file_path: String) -> Vec<String>;

    fn all_modules_ast(&self) -> Result<Vec<UntypedModule>, Vec<Error>>;
}

fn all_modules_ast(db: &impl Modules) -> Result<Vec<UntypedModule>, Vec<Error>> {
    let mut errors = vec![];
    let mut modules = vec![];
    for src in db.sources(()).drain() {
        match db.module_ast(src) {
            Ok(module) => {
                modules.push(module);
            },
            Err(e) => {
                errors.push(e);
            }
        }
    }

    if errors.is_empty() {
        Err(errors)
    } else {
        Ok(modules)
    }
}

pub fn module_ast(
    db: &impl Modules,
    file_path: String,
) -> Result<UntypedModule, Error> {
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
                Statement::Import {
                    module,
                    ..
                } => Some(module.join("/")),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}