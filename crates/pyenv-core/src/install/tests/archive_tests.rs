// ./crates/pyenv-core/src/install/tests/archive_tests.rs
//! Archive-extraction and alias/wrapper regression tests for install helpers.

use std::fs;
use std::io::Write;

use bzip2::Compression;
use bzip2::write::BzEncoder;
use tar::Builder;
use tempfile::TempDir;
use zip::write::FileOptions;

use super::super::archive::{
    extract_root_archive, extract_tar_root_archive, extract_tools_archive,
};
use super::super::report::pip_wrapper_names;
use super::super::runtime_support::ensure_unix_runtime_aliases;

#[test]
fn extract_tools_archive_strips_prefix() {
    let temp = TempDir::new().expect("tempdir");
    let archive_path = temp.path().join("test.nupkg");
    let output_dir = temp.path().join("out");
    let file = fs::File::create(&archive_path).expect("archive");
    let mut writer = zip::ZipWriter::new(file);
    let options = FileOptions::<()>::default();
    writer.add_directory("tools/Lib/", options).expect("dir");
    writer
        .start_file("tools/python.exe", options)
        .expect("file");
    writer.write_all(b"python").expect("write");
    writer
        .start_file("tools/Lib/test.py", options)
        .expect("file");
    writer.write_all(b"pass").expect("write");
    writer.finish().expect("finish");

    extract_tools_archive(&archive_path, &output_dir).expect("extract");

    assert!(output_dir.join("python.exe").is_file());
    assert!(output_dir.join("Lib").join("test.py").is_file());
    assert!(!output_dir.join("tools").exists());
}

#[test]
fn extract_root_archive_strips_top_level_directory() {
    let temp = TempDir::new().expect("tempdir");
    let archive_path = temp.path().join("test.zip");
    let output_dir = temp.path().join("out");
    let file = fs::File::create(&archive_path).expect("archive");
    let mut writer = zip::ZipWriter::new(file);
    let options = FileOptions::<()>::default();
    writer.add_directory("runtime/Lib/", options).expect("dir");
    writer
        .start_file("runtime/python.exe", options)
        .expect("file");
    writer.write_all(b"python").expect("write");
    writer
        .start_file("runtime/Lib/test.py", options)
        .expect("file");
    writer.write_all(b"pass").expect("write");
    writer.finish().expect("finish");

    extract_root_archive(&archive_path, &output_dir).expect("extract");

    assert!(output_dir.join("python.exe").is_file());
    assert!(output_dir.join("Lib").join("test.py").is_file());
    assert!(!output_dir.join("runtime").exists());
}

#[test]
fn extract_tar_root_archive_strips_top_level_directory() {
    let temp = TempDir::new().expect("tempdir");
    let archive_path = temp.path().join("test.tar.bz2");
    let output_dir = temp.path().join("out");
    let file = fs::File::create(&archive_path).expect("archive");
    let encoder = BzEncoder::new(file, Compression::best());
    let mut builder = Builder::new(encoder);

    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Directory);
    header.set_mode(0o755);
    header.set_size(0);
    header.set_cksum();
    builder
        .append_data(&mut header, "runtime/Lib", std::io::empty())
        .expect("dir");

    let mut file_header = tar::Header::new_gnu();
    file_header.set_mode(0o755);
    file_header.set_size(6);
    file_header.set_cksum();
    builder
        .append_data(&mut file_header, "runtime/bin/pypy3", &b"python"[..])
        .expect("binary");

    let mut lib_header = tar::Header::new_gnu();
    lib_header.set_mode(0o644);
    lib_header.set_size(4);
    lib_header.set_cksum();
    builder
        .append_data(&mut lib_header, "runtime/Lib/test.py", &b"pass"[..])
        .expect("lib");
    let encoder = builder.into_inner().expect("encoder");
    encoder.finish().expect("finish");

    extract_tar_root_archive(&archive_path, &output_dir).expect("extract");

    assert!(output_dir.join("bin").join("pypy3").is_file());
    assert!(output_dir.join("Lib").join("test.py").is_file());
    assert!(!output_dir.join("runtime").exists());
}

#[test]
fn pip_wrapper_names_include_versioned_commands() {
    assert_eq!(
        pip_wrapper_names("3.13.12"),
        vec!["pip".to_string(), "pip3".to_string(), "pip3.13".to_string()]
    );
}

#[test]
fn unix_runtime_aliases_create_python_and_pip_links() {
    let temp = TempDir::new().expect("tempdir");
    let prefix = temp.path().join("runtime");
    let bin = prefix.join("bin");
    fs::create_dir_all(&bin).expect("bin");
    fs::write(bin.join("python3"), "").expect("python3");
    fs::write(bin.join("pip3"), "").expect("pip3");

    ensure_unix_runtime_aliases(&prefix, "3.12.10").expect("aliases");

    assert!(bin.join("python").exists());
    assert!(bin.join("pip").exists());
}
