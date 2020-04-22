mod analyze;
mod codegen;
mod modules;
mod sources;

use salsa::{Database, Runtime};

use self::analyze::AnalyzedStorage;
use self::codegen::CodeGenStorage;
use self::modules::ModulesStorage;
use self::sources::SourcesStorage;

pub use self::analyze::Analyzed;
pub use self::codegen::CodeGen;
pub use self::modules::Modules;
pub use self::sources::Sources;

#[salsa::database(SourcesStorage, ModulesStorage, AnalyzedStorage, CodeGenStorage)]
#[derive(Default)]
pub struct GleamDatabase {
    runtime: Runtime<GleamDatabase>,
}

impl Database for GleamDatabase {
    fn salsa_runtime(&self) -> &Runtime<Self> {
        &self.runtime
    }

    fn salsa_runtime_mut(&mut self) -> &mut Runtime<Self> {
        &mut self.runtime
    }
}
