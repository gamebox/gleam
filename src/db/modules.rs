use super::sources::Sources;
use crate::error::Error;
use crate::project::Module;

#[salsa::query_group(ModulesStorage)]
pub trait Modules: Sources {
    fn module_ast(&self, file_path: String) -> Result<Module, Error>;

    fn dependencies(&self, file_path: String) -> Vec<(String, crate::ast::SrcSpan)>;

    fn dependents(&self, file_path: String) -> Vec<String>;

    fn all_modules_ast(&self) -> Result<Vec<Module>, Error>;
}

fn all_modules_ast(db: &impl Modules) -> Result<Vec<Module>, Error> {
    let mut modules = vec![];
    for src in db.sources(()).drain() {
        let module = db.module_ast(src)?;
        modules.push(module);
    }

    Ok(modules)
}

pub fn module_ast(db: &impl Modules, file_path: String) -> Result<Module, Error> {
    let all_sources = db.sources(());
    if let None = all_sources.get(&file_path) {
        use crate::error::*;
        return Err(Error::FileIO {
            action: FileIOAction::Read,
            kind: FileKind::File,
            path: std::path::PathBuf::new(),
            err: Some(format!("Unable to resolve module {}", file_path)),
        });
    }
    let source = db.source_file(file_path);
    let (stripped, comments) = crate::parser::strip_extra(&source.src);
    let mut module = crate::grammar::ModuleParser::new()
        .parse(&stripped)
        .map_err(|e| crate::error::Error::Parse {
            path: source.path.clone(),
            src: source.src.clone(),
            error: e.map_token(|crate::grammar::Token(a, b)| (a, b.to_string())),
        })?;

    crate::project::attach_doc_comments(&mut module, &comments.doc_comments);
    let name = crate::project::create_module_name(&source);
    module.name = name.split("/").into_iter().map(|c| c.to_string()).collect();
    Ok(Module {
        src: source.src.clone(),
        path: source.path.clone(),
        origin: source.origin.clone(),
        source_base_path: source.source_base_path.clone(),
        module,
    })
}

pub fn dependencies(db: &impl Modules, file_path: String) -> Vec<(String, crate::ast::SrcSpan)> {
    match db.module_ast(file_path) {
        Ok(module) => module.dependencies().into_iter().collect(),
        _ => vec![],
    }
}

pub fn dependents(db: &impl Modules, file_path: String) -> Vec<String> {
    db.all_sources()
        .into_iter()
        .map(|source| crate::project::create_module_name(&source))
        .filter(|source| {
            db.dependencies(source.clone())
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<String>>()
                .contains(&file_path)
        })
        .collect()
}
