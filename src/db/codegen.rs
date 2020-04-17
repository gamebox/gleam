use super::analyze::Analyzed;
use crate::project::OutputFile;

#[salsa::query_group(CodeGenStorage)]
pub trait CodeGen: Analyzed {
    fn generate_project_code(&self) -> Vec<OutputFile>;
}

fn generate_project_code(db: &impl CodeGen) -> Vec<OutputFile> {
    let mut files = vec![];
    let _ = db.all_modules_analyzed()
        .map(|analyzed| crate::project::generate_erlang(&analyzed, &mut files));
    files
}