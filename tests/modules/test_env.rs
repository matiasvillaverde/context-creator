use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

pub(crate) fn prepend_to_command_path(cmd: &mut Command, dir: &Path) {
    set_command_path(cmd, path_with_prepended_dir(dir));
}

pub(crate) fn set_command_path(cmd: &mut Command, path: impl Into<OsString>) {
    let path = path.into();
    cmd.env("PATH", &path);

    #[cfg(windows)]
    cmd.env("Path", &path);
}

fn path_with_prepended_dir(dir: &Path) -> OsString {
    let mut paths = vec![dir.to_path_buf()];
    let current_path = std::env::var_os("PATH").unwrap_or_default();
    paths.extend(std::env::split_paths(&current_path));

    std::env::join_paths(paths).expect("mock tool path should be valid")
}
