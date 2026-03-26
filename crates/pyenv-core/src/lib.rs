// ./crates/pyenv-core/src/lib.rs
//! Native core behavior for pyenv-compatible version resolution and configuration.

mod catalog;
mod command;
mod config;
mod context;
mod doctor;
mod error;
mod executable;
mod http;
mod install;
mod manage;
mod meta;
mod plugin;
mod process;
mod runtime;
mod self_update;
mod shell;
mod shim;
mod venv;
mod venv_compat;
mod venv_paths;
mod version;

pub use process::CommandExt;

pub use catalog::{
    CatalogEntry, CatalogGroup, CatalogSourceKind, InstallListOptions, cmd_install_list,
    cmd_latest, compare_version_names, installed_version_names, known_version_names,
    latest_installed_version, latest_known_version,
};
pub use command::CommandReport;
pub use config::{
    AppConfig, InstallConfig, RegistryMode, RuntimeArch, StorageConfig, VenvConfig, WindowsConfig,
    cmd_config_get, cmd_config_path, cmd_config_set, cmd_config_show, config_path,
    resolve_cache_dir,
};
pub use context::{AppContext, is_pyenv_win_root, resolve_dir, resolve_root};
pub use doctor::{DoctorFix, DoctorFixOutcome, apply_doctor_fixes, cmd_doctor, doctor_fix_plan};
pub use error::PyenvError;
pub use executable::{cmd_whence, cmd_which};
pub use install::{
    InstallCommandOptions, InstallOutcome, InstallPlan, cmd_available, cmd_install,
    install_runtime_plan, resolve_install_plan,
};
pub use manage::{VersionsCommandOptions, cmd_prefix, cmd_uninstall, cmd_versions};
pub use meta::{
    EnvironmentStatus, ManagedVenvSummary, build_environment_status, cmd_commands, cmd_completions,
    cmd_help, cmd_prompt, cmd_shims, cmd_status,
};
pub use plugin::{HookResult, cmd_external, cmd_hooks};
pub use runtime::BASE_VENV_DIR_NAME;
pub use self_update::{SelfUpdateOptions, cmd_self_uninstall, cmd_self_update};
pub use shell::{
    InitCommandOptions, cmd_activate, cmd_deactivate, cmd_init, cmd_sh_activate, cmd_sh_cmd,
    cmd_sh_deactivate, cmd_sh_rehash, cmd_sh_shell, cmd_shell, cmd_virtualenv_init,
};
pub use shim::{cmd_exec, cmd_rehash};
pub use venv::{
    ManagedVenvInfo, VenvUseScope, cmd_venv_create, cmd_venv_delete, cmd_venv_info, cmd_venv_list,
    cmd_venv_rename, cmd_venv_use, list_managed_venvs, resolve_managed_venv,
};
pub use venv_compat::{
    cmd_virtualenv, cmd_virtualenv_delete, cmd_virtualenv_prefix, cmd_virtualenvs,
};
pub use version::{
    SelectedVersions, VersionOrigin, cmd_global, cmd_local, cmd_root, cmd_version,
    cmd_version_file, cmd_version_file_read, cmd_version_file_write, cmd_version_name,
    cmd_version_origin, find_local_version_file, installed_version_dir, read_version_file,
    resolve_selected_versions, version_file_path, version_origin,
};
