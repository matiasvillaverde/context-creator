use std::ffi::OsString;
use std::process::Command;

pub(crate) fn set_command_path(cmd: &mut Command, path: impl Into<OsString>) {
    let path = path.into();

    #[cfg(windows)]
    {
        cmd.env_remove("PATH");
        cmd.env_remove("Path");
        cmd.env("Path", &path);
    }

    #[cfg(not(windows))]
    cmd.env("PATH", &path);
}
