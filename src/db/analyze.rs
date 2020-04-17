use super::modules::Modules;
use crate::project::Analysed;
use crate::error::Error;

#[salsa::query_group(AnalyzedStorage)]
pub trait Analyzed: Modules {
    fn all_modules_analyzed(&self) -> Result<Vec<Analysed>, Error>;
}

fn all_modules_analyzed(db: &impl Analyzed) -> Result<Vec<Analysed>, Error> {
    let inputs = db.all_sources();
    crate::project::analysed(inputs)
}