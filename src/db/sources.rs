use salsa::{Database};
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

pub fn all_sources(db: &impl Sources) -> Vec<Input> {
    let mut sources = db.sources(());
    sources.drain().map(|src| db.source_file(src)).collect()
}