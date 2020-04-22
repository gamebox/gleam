use super::analyze::Analyzed;
use crate::error::Error;
use crate::project::OutputFile;

#[salsa::query_group(CodeGenStorage)]
pub trait CodeGen: Analyzed {
    fn generate_project_code(&self) -> Result<Vec<OutputFile>, Error>;
    fn invalidate_module(&self, file_path: String) -> Result<Vec<OutputFile>, Error>;
    fn generate_new_module(&self, file_path: String) -> Result<Vec<OutputFile>, Error>;
}

fn generate_project_code(db: &impl CodeGen) -> Result<Vec<OutputFile>, Error> {
    let mut files = vec![];
    match db
        .all_modules_analyzed()
        .map(|analyzed| crate::project::generate_erlang(&analyzed, &mut files))
    {
        Ok(()) => Ok(files),
        Err(e) => Err(e),
    }
}

fn invalidate_module(db: &impl CodeGen, file_path: String) -> Result<Vec<OutputFile>, Error> {
    let _ = db.check_dependents(file_path.clone());
    let mut files = vec![];

    let analyzed = db.analyzed_module(file_path.clone())?;
    let _ = db.available_symbols(file_path)?;
    crate::project::generate_erlang(&[analyzed], &mut files);
    Ok(files)
}

fn generate_new_module(db: &impl CodeGen, file_path: String) -> Result<Vec<OutputFile>, Error> {
    let mut files = vec![];
    let analyzed = db.analyzed_module(file_path)?;
    crate::project::generate_erlang(&[analyzed], &mut files);
    Ok(files)
}
