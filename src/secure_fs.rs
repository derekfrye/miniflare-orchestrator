use cap_std::ambient_authority;
use cap_std::fs::{Dir, DirBuilder, DirBuilderExt, OpenOptions, OpenOptionsExt};
use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::path::{Component, Path};

pub(crate) fn open_ambient_dir(path: &Path) -> io::Result<Dir> {
    Dir::open_ambient_dir(path, ambient_authority())
}

pub(crate) fn create_ambient_private_dir_all(path: &Path) -> io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(path)
}

pub(crate) fn clear_dir_contents(dir: &Dir) -> io::Result<()> {
    for entry in dir.entries()? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            let child = entry.open_dir()?;
            clear_dir_contents(&child)?;
            child.remove_open_dir()?;
        } else {
            entry.remove_file()?;
        }
    }
    Ok(())
}

pub(crate) fn create_private_dir_all(root: &Dir, path: &Path) -> io::Result<Dir> {
    let mut dir = root.try_clone()?;
    let mut builder = DirBuilder::new();
    builder.mode(0o700);

    for component in normal_components(path)? {
        match dir.create_dir_with(component, &builder) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
        dir = dir.open_dir(component)?;
    }

    Ok(dir)
}

pub(crate) fn create_private_file_new(dir: &Dir, name: &OsStr, contents: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true).mode(0o600);
    let mut file = dir.open_with(Path::new(name), &options)?;
    file.write_all(contents)
}

pub(crate) fn write_private_file(dir: &Dir, name: &OsStr, contents: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true).mode(0o600);
    let mut file = dir.open_with(Path::new(name), &options)?;
    file.write_all(contents)
}

pub(crate) fn open_append_private_file(dir: &Dir, name: &str) -> io::Result<std::fs::File> {
    let mut options = OpenOptions::new();
    options.create(true).append(true).mode(0o600);
    Ok(dir.open_with(name, &options)?.into_std())
}

pub(crate) fn read_file_to_end(file: &mut cap_std::fs::File) -> io::Result<Vec<u8>> {
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    Ok(contents)
}

fn normal_components(path: &Path) -> io::Result<Vec<&OsStr>> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => components.push(part),
            Component::CurDir => {}
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("path contains an invalid segment: {}", path.display()),
                ));
            }
        }
    }
    Ok(components)
}
