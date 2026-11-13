// ./xtask/src/locale.rs
//! `cargo xtask new-locale <tag>`: seed a new Fluent locale from `en-US`.
//!
//! Replaces `scripts/i18n/new-locale.{sh,ps1}`. The new catalog is a copy of every `en-US` file
//! with the path comments retargeted; translation and seeding stay in `scripts/i18n`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Args;

use crate::repo_root;

#[derive(Args)]
pub struct NewLocaleArgs {
    /// BCP 47 language tag for the new locale, for example `fr-CA`.
    tag: String,
}

/// Traditional Chinese tags the project maps onto `zh-CN` instead of keeping a catalog for.
const FOLDED_INTO_ZH_CN: &[&str] = &["zh-tw", "zh-hant", "zh-hk", "zh-mo"];

pub fn run(args: &NewLocaleArgs) -> Result<()> {
    let root = repo_root();
    let dest = create(&root, &args.tag)?;
    println!(
        "Created {} from en-US. Add scripts/i18n/overlays/{tag}.json, list the tag in \
         seed_locales.py LOCALES, translate, seed, then run cargo xtask policy i18n",
        dest.display(),
        tag = args.tag,
    );
    Ok(())
}

fn validate(tag: &str) -> Result<()> {
    if tag.is_empty() {
        bail!("a BCP 47 locale tag is required");
    }
    if FOLDED_INTO_ZH_CN.contains(&tag.to_ascii_lowercase().as_str()) {
        bail!("Traditional Chinese is mapped to zh-CN. Do not add a separate catalog.");
    }
    // The tag becomes a directory name, so keep it to what a language tag can contain.
    let well_formed = tag
        .split('-')
        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric()));
    if !well_formed {
        bail!("`{tag}` is not a BCP 47 tag (letters, digits and single hyphens only)");
    }
    Ok(())
}

/// Copy `locales/en-US/*.ftl` to `locales/<tag>/`, returning the new directory.
pub fn create(root: &Path, tag: &str) -> Result<PathBuf> {
    validate(tag)?;
    let source = root.join("locales").join("en-US");
    let dest = root.join("locales").join(tag);
    if dest.exists() {
        bail!("locale {tag} already exists at {}", dest.display());
    }
    fs::create_dir_all(&dest).with_context(|| format!("creating {}", dest.display()))?;

    let mut copied = 0;
    for entry in fs::read_dir(&source).with_context(|| format!("reading {}", source.display()))? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("ftl") {
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?
            .replace("./locales/en-US/", &format!("./locales/{tag}/"));
        let name = path.file_name().expect("a directory entry has a file name");
        fs::write(dest.join(name), text)?;
        copied += 1;
    }
    if copied == 0 {
        bail!("no .ftl files found in {}", source.display());
    }
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let en = dir.path().join("locales/en-US");
        fs::create_dir_all(&en).unwrap();
        fs::write(
            en.join("cli.ftl"),
            "# ./locales/en-US/cli.ftl\nhello = Hi\n",
        )
        .unwrap();
        fs::write(en.join("notes.txt"), "not a catalog").unwrap();
        dir
    }

    #[test]
    fn copies_catalogs_and_retargets_path_comments() {
        let dir = fixture();
        let dest = create(dir.path(), "fr-CA").unwrap();
        let text = fs::read_to_string(dest.join("cli.ftl")).unwrap();
        assert_eq!(text, "# ./locales/fr-CA/cli.ftl\nhello = Hi\n");
        assert!(!dest.join("notes.txt").exists());
    }

    #[test]
    fn refuses_existing_locales() {
        let dir = fixture();
        create(dir.path(), "de").unwrap();
        let error = create(dir.path(), "de").unwrap_err().to_string();
        assert!(error.contains("already exists"), "{error}");
    }

    #[test]
    fn folds_traditional_chinese_into_zh_cn() {
        let dir = fixture();
        for tag in ["zh-TW", "zh-Hant", "zh-hk", "zh-MO"] {
            let error = create(dir.path(), tag).unwrap_err().to_string();
            assert!(error.contains("zh-CN"), "{tag}: {error}");
        }
        assert!(!dir.path().join("locales/zh-TW").exists());
    }

    #[test]
    fn rejects_tags_that_are_not_directory_safe() {
        let dir = fixture();
        for tag in ["", "../etc", "fr/CA", "fr--CA", "-fr", "fr CA"] {
            assert!(
                create(dir.path(), tag).is_err(),
                "{tag:?} should be refused"
            );
        }
    }
}
