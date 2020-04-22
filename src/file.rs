use crate::error::{Error, FileIOAction, FileKind};
use crate::project::OutputFile;
use std::fs::File;
use std::io::Write;

pub fn write_file(file: OutputFile) -> Result<(), Error> {
    let OutputFile { path, text } = file;

    let dir_path = path.parent().ok_or_else(|| Error::FileIO {
        action: FileIOAction::FindParent,
        kind: FileKind::Directory,
        path: path.clone(),
        err: None,
    })?;

    std::fs::create_dir_all(dir_path).map_err(|e| Error::FileIO {
        action: FileIOAction::Create,
        kind: FileKind::Directory,
        path: dir_path.to_path_buf(),
        err: Some(e.to_string()),
    })?;

    let mut f = File::create(&path).map_err(|e| Error::FileIO {
        action: FileIOAction::Create,
        kind: FileKind::File,
        path: path.clone(),
        err: Some(e.to_string()),
    })?;

    f.write_all(text.as_bytes()).map_err(|e| Error::FileIO {
        action: FileIOAction::WriteTo,
        kind: FileKind::File,
        path: path.clone(),
        err: Some(e.to_string()),
    })?;
    Ok(())
}
