mod sources;
mod modules;
mod analyze;
mod codegen;

use salsa::{Database, Runtime};

use self::sources::SourcesStorage;
use self::modules::ModulesStorage;
use self::analyze::AnalyzedStorage;
use self::codegen::CodeGenStorage;

pub use self::sources::Sources;
pub use self::modules::Modules;
pub use self::analyze::Analyzed;
pub use self::codegen::CodeGen;

#[salsa::database(
    SourcesStorage,
    ModulesStorage,
    AnalyzedStorage,
    CodeGenStorage
)]
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
