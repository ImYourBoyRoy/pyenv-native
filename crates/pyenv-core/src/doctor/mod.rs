// ./crates/pyenv-core/src/doctor/mod.rs
//! Health and diagnostics reporting for common pyenv-native configuration issues.

mod checks;
mod fixes;
mod helpers;
mod report;
mod tests;
mod types;

pub use fixes::{apply_doctor_fixes, doctor_fix_plan};
pub use report::cmd_doctor;
pub use types::{DoctorFix, DoctorFixOutcome};
