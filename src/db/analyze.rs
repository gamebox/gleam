use super::modules::Modules;
use crate::error::Error;
use crate::pretty::Documentable;
use crate::project::Analysed;
use std::collections::HashMap;

#[salsa::query_group(AnalyzedStorage)]
pub trait Analyzed: Modules {
    fn all_modules_analyzed(&self) -> Result<Vec<Analysed>, Error>;
    fn analyzed_module(&self, file_path: String) -> Result<Analysed, Error>;
    fn check_dependents(&self, file_path: String) -> Result<(), Error>;
    fn available_symbols(&self, file_path: String) -> Result<(), Error>;
}

fn check_dependents(db: &impl Analyzed, file_path: String) -> Result<(), Error> {
    for module_name in db.dependents(file_path.clone()).into_iter() {
        let _ = db.analyzed_module(module_name)?;
    }

    Ok(())
}

fn all_modules_analyzed(db: &impl Analyzed) -> Result<Vec<Analysed>, Error> {
    db.all_sources()
        .into_iter()
        .map(|input| db.analyzed_module(crate::project::create_module_name(&input)))
        .collect()
}

fn analyzed_module(db: &impl Analyzed, file_path: String) -> Result<Analysed, Error> {
    let deps = db.dependencies(file_path.clone());
    let mut module_type_info = HashMap::with_capacity(deps.len());
    let crate::project::Module {
        path,
        src,
        origin,
        source_base_path,
        module,
    } = db.module_ast(file_path.clone())?;
    for (dep, location) in deps.into_iter() {
        let module = db.analyzed_module(dep.clone()).map_err(|e| match e {
            Error::FileIO { .. } => {
                let modules: Vec<String> = db.sources(()).drain().collect();
                Error::UnknownImport {
                    module: file_path.clone(),
                    import: dep.clone(),
                    location,
                    path: path.clone(),
                    src: src.clone(),
                    modules,
                }
            }
            _ => e,
        })?;
        module_type_info.insert(dep, module.type_info.clone());
    }
    let ast = crate::typ::infer_module(module, &module_type_info).map_err(|error| Error::Type {
        path,
        src,
        error,
    })?;

    let type_info = ast.type_info.clone();

    Ok(Analysed {
        name: ast.name.clone(),
        origin,
        source_base_path,
        ast,
        type_info,
    })
}

fn available_symbols(db: &impl Analyzed, file_path: String) -> Result<(), Error> {
    println!("{:#?}", db.analyzed_module(file_path));
    // let Analysed {
    //     type_info: crate::typ::Module {
    //         name,
    //         types,
    //         values,
    //         accessors,
    //     },
    //     ..
    // } = db.analyzed_module(file_path)?;
    // println!("Module {}", name.join("/"));
    // println!("Types\n==========================");
    // types.into_iter().for_each(|(name, typ)| {
    //     use crate::typ::*;
    //     let printer = crate::typ::pretty::Printer::new();
    //     let TypeConstructor {
    //         public,
    //         origin,
    //         module,
    //         parameters,
    //         typ
    //     } = typ;
    //     println!("{} {} {} {}", public, module.join("/"), name, printer.pretty_print(&typ, 0));
    // });

    // values.into_iter().for_each(|(name, val)| {
    //     let crate::typ::ValueConstructor {
    //         public,
    //         typ,
    //         variant,
    //     } = val;

    //     println!("")
    // });
    Ok(())
}
