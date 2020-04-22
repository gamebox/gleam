#[cfg(test)]
mod tests;

use crate::ast::{SrcSpan, TypedModule};
use crate::error::{Error, FileIOAction, FileKind, GleamExpect};
use crate::parser::{self, Comment};
use crate::typ;
use source_tree::SourceTree;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug)]
pub struct SourceCollection {
    dependency_dirs: Vec<PathBuf>,
    source_dir: PathBuf,
    test_dir: PathBuf,
}

impl SourceCollection {
    pub fn new(source_dir: PathBuf, test_dir: PathBuf, dependency_dirs: Vec<PathBuf>) -> Self {
        SourceCollection {
            source_dir,
            test_dir,
            dependency_dirs,
        }
    }

    pub fn sources(&self, project_name: &String) -> Result<Vec<Input>, Error> {
        let mut srcs = vec![];

        for project_dir in self
            .dependency_dirs
            .iter()
            .filter_map(|d| std::fs::read_dir(d).ok())
            .flat_map(|d| d.filter_map(Result::ok))
            .map(|d| d.path())
            .filter(|p| {
                p.file_name().and_then(|os_string| os_string.to_str()) != Some(project_name)
            })
        {
            collect_source(project_dir.join("src"), ModuleOrigin::Dependency, &mut srcs)?;
        }

        // Collect source code from top level project
        crate::project::collect_source(self.source_dir.clone(), ModuleOrigin::Src, &mut srcs)?;
        crate::project::collect_source(self.test_dir.clone(), ModuleOrigin::Test, &mut srcs)?;

        Ok(srcs)
    }

    pub fn dirs(&self) -> Vec<PathBuf> {
        let mut ds = self.dependency_dirs.clone();
        ds.push(self.source_dir.clone());
        ds.push(self.test_dir.clone());
        ds
    }

    pub fn origin_of(&self, path: PathBuf) -> Option<(PathBuf, ModuleOrigin)> {
        if path.starts_with(self.source_dir.clone()) {
            Some((self.source_dir.clone(), ModuleOrigin::Src))
        } else if path.starts_with(self.test_dir.clone()) {
            Some((self.test_dir.clone(), ModuleOrigin::Test))
        } else {
            self.dependency_dirs
                .clone()
                .into_iter()
                .find(|dir| path.starts_with(dir))
                .map(|dir| (dir, ModuleOrigin::Dependency))
        }
    }
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct Input {
    pub source_base_path: PathBuf,
    pub path: PathBuf,
    pub src: String,
    pub origin: ModuleOrigin,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Analysed {
    pub ast: TypedModule,
    pub name: Vec<String>,
    pub origin: ModuleOrigin,
    pub type_info: typ::Module,
    pub source_base_path: PathBuf,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct OutputFile {
    pub text: String,
    pub path: PathBuf,
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum ModuleOrigin {
    Src,
    Test,
    Dependency,
}

impl ModuleOrigin {
    pub fn dir_name(&self) -> &'static str {
        match self {
            ModuleOrigin::Src | ModuleOrigin::Dependency => "src",
            ModuleOrigin::Test => "test",
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Module {
    pub src: String,
    pub path: PathBuf,
    pub source_base_path: PathBuf,
    pub origin: ModuleOrigin,
    pub module: crate::ast::UntypedModule,
}

impl Module {
    pub fn dependencies(&self) -> Vec<(String, SrcSpan)> {
        self.module.dependencies()
    }
}

pub fn analysed(inputs: Vec<Input>) -> Result<Vec<Analysed>, Error> {
    let module_count = inputs.len();
    let mut source_tree = SourceTree::new(inputs)?;
    let mut modules_type_infos = HashMap::new();
    let mut compiled_modules = Vec::with_capacity(module_count);

    struct Out {
        source_base_path: PathBuf,
        name_string: String,
        name: Vec<String>,
        origin: ModuleOrigin,
        ast: TypedModule,
    }

    for Module {
        src,
        path,
        module,
        origin,
        source_base_path,
    } in source_tree.consume()?
    {
        let name = module.name.clone();
        let name_string = module.name_string();

        println!("Compiling {}", name_string);

        let ast = crate::typ::infer_module(module, &modules_type_infos)
            .map_err(|error| Error::Type { path, src, error })?;

        modules_type_infos.insert(name_string.clone(), ast.type_info.clone());

        compiled_modules.push(Out {
            name,
            name_string,
            source_base_path,
            origin,
            ast,
        });
    }

    Ok(compiled_modules
        .into_iter()
        .map(|out| {
            let Out {
                name,
                source_base_path,
                name_string,
                origin,
                ast,
            } = out;
            Analysed {
                ast,
                name,
                source_base_path,
                origin,
                type_info: modules_type_infos
                    .remove(&name_string)
                    .gleam_expect("project::compile(): Merging module type info"),
            }
        })
        .collect())
}

pub fn generate_erlang(analysed: &[Analysed], files: &mut Vec<OutputFile>) {
    for Analysed {
        name,
        origin,
        source_base_path,
        ast,
        ..
    } in analysed
    {
        let gen_dir = source_base_path
            .parent()
            .unwrap()
            .join("gen")
            .join(origin.dir_name());
        let erl_module_name = name.join("@");

        for (name, text) in crate::erl::records(&ast).into_iter() {
            files.push(OutputFile {
                path: gen_dir.join(format!("{}_{}.hrl", erl_module_name, name)),
                text,
            })
        }

        files.push(OutputFile {
            path: gen_dir.join(format!("{}.erl", erl_module_name)),
            text: crate::erl::module(&ast),
        });
    }
}

pub fn collect_source(
    src_dir: PathBuf,
    origin: ModuleOrigin,
    srcs: &mut Vec<Input>,
) -> Result<(), Error> {
    let src_dir = match src_dir.canonicalize() {
        Ok(d) => d,
        Err(_) => return Ok(()),
    };
    let is_gleam_path = |e: &walkdir::DirEntry| {
        use regex::Regex;
        lazy_static! {
            static ref RE: Regex = Regex::new("^([a-z_]+(/|\\\\))*[a-z_]+\\.gleam$")
                .gleam_expect("project::collect_source() RE regex");
        }

        RE.is_match(
            e.path()
                .strip_prefix(&*src_dir)
                .gleam_expect("project::collect_source(): strip_prefix")
                .to_str()
                .unwrap_or(""),
        )
    };

    for dir_entry in walkdir::WalkDir::new(src_dir.clone())
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(is_gleam_path)
    {
        let src = std::fs::read_to_string(dir_entry.path()).map_err(|err| Error::FileIO {
            action: FileIOAction::Read,
            kind: FileKind::File,
            path: dir_entry.path().to_path_buf(),
            err: Some(err.to_string()),
        })?;

        srcs.push(Input {
            path: dir_entry
                .path()
                .canonicalize()
                .gleam_expect("project::collect_source(): path canonicalize"),
            source_base_path: src_dir.clone(),
            origin: origin.clone(),
            src,
        })
    }
    Ok(())
}

pub fn create_module_name(input: &Input) -> String {
    input
        .path
        .strip_prefix(input.source_base_path.clone())
        .unwrap()
        .parent()
        .unwrap()
        .join(input.path.file_stem().unwrap())
        .to_str()
        .unwrap()
        .to_string()
        .replace("\\", "/")
}

pub fn attach_doc_comments<'a, A, B>(
    module: &mut crate::ast::Module<A, B>,
    mut comments: &'a [Comment<'a>],
) {
    for statement in &mut module.statements {
        let location = statement.location();
        // TODO: be more fine grained with how we apply the comments.
        // i.e. Apply them to custom type constructors.
        let (doc, rest) = parser::take_before(comments, location.end);
        comments = rest;
        statement.put_doc(doc);
    }
}

pub fn is_gleam_path(path: PathBuf, src_dir: PathBuf) -> bool {
    use regex::Regex;
    lazy_static! {
        static ref RE: Regex = Regex::new("^([a-z_]+(/|\\\\))*[a-z_]+\\.gleam$")
            .gleam_expect("project::collect_source() RE regex");
    }

    RE.is_match(
        path.strip_prefix(&*src_dir)
            .gleam_expect("project::collect_source(): strip_prefix")
            .to_str()
            .unwrap_or(""),
    )
}

pub fn create_input_from_source(
    path: PathBuf,
    src: String,
    source_collection: &SourceCollection,
) -> Option<Input> {
    source_collection
        .origin_of(path.clone())
        .map(|(source_base_path, origin)| Input {
            path: path
                .canonicalize()
                .gleam_expect("project::create_input_from_source(): path canonicalize"),
            source_base_path,
            origin,
            src,
        })
}
