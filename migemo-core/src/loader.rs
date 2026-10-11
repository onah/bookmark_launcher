//! Locating and loading the SKK dictionary for the command-line tools.
//!
//! Every tool in this workspace is driven by a shell on every keystroke, so
//! they share one data directory (a dictionary downloaded once works for all
//! of them) and one compiled cache next to the source dictionary.

use super::dictionary::SkkDictionary;
use std::path::{Path, PathBuf};

/// `SKK-JISYO.L` in the shared data directory, or `None` if the platform has
/// no home directory to put it under.
pub fn default_dictionary_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "onah", "bookmark_launcher")
        .map(|proj| proj.data_dir().join("SKK-JISYO.L"))
}

/// Load the dictionary at `explicit_path`, or at [`default_dictionary_path`]
/// when none is given.
///
/// A missing or unreadable dictionary degrades to an empty one (kana and
/// ASCII matching still work) rather than failing, so a shell driving a tool
/// on every keystroke never sees it crash.
///
/// Parsing the ~170k-entry SKK-JISYO.L text takes ~100ms (see
/// `examples/bench_dict_load.rs`), too slow to redo on every keystroke. A
/// compiled cache next to the source file cuts that to microseconds; it is
/// rebuilt whenever missing, corrupt, or older than the source dictionary.
pub fn load_dictionary(explicit_path: Option<&Path>) -> SkkDictionary {
    let Some(source_path) = explicit_path
        .map(PathBuf::from)
        .or_else(default_dictionary_path)
    else {
        return SkkDictionary::from_bytes(&[]);
    };

    let cache_path = compiled_cache_path(&source_path);
    if let Some(dict) = load_compiled_cache(&cache_path, &source_path) {
        return dict;
    }

    let Ok(bytes) = std::fs::read(&source_path) else {
        return SkkDictionary::from_bytes(&[]);
    };

    let dict = SkkDictionary::from_bytes(&bytes);
    // Best-effort: a read-only data dir or a race with another concurrent
    // process must not stop this run from working.
    let _ = std::fs::write(&cache_path, dict.compile());
    dict
}

fn compiled_cache_path(source_path: &Path) -> PathBuf {
    let mut file_name = source_path.file_name().unwrap_or_default().to_os_string();
    file_name.push(".compiled");
    source_path.with_file_name(file_name)
}

/// `None` on any problem (missing files, unreadable, stale, corrupt) so the
/// caller always falls back to rebuilding from the source dictionary.
fn load_compiled_cache(cache_path: &Path, source_path: &Path) -> Option<SkkDictionary> {
    let cache_modified = std::fs::metadata(cache_path)
        .and_then(|m| m.modified())
        .ok()?;
    let source_modified = std::fs::metadata(source_path)
        .and_then(|m| m.modified())
        .ok()?;
    if cache_modified < source_modified {
        return None;
    }

    let bytes = std::fs::read(cache_path).ok()?;
    SkkDictionary::from_compiled_bytes(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh directory under the system temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("migemo-core-loader-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn write_source_dictionary(dir: &Path) -> PathBuf {
        let text = "\
;; okuri-nasi entries.
かんり /管理/
げんご /言語/
";
        let (bytes, _, had_errors) = encoding_rs::EUC_JP.encode(text);
        assert!(!had_errors);
        let path = dir.join("SKK-JISYO.L");
        std::fs::write(&path, bytes).expect("write source dictionary");
        path
    }

    #[test]
    fn compiled_cache_path_appends_suffix() {
        assert_eq!(
            compiled_cache_path(Path::new("/data/SKK-JISYO.L")),
            PathBuf::from("/data/SKK-JISYO.L.compiled")
        );
    }

    #[test]
    fn missing_dictionary_degrades_to_empty() {
        let dir = TempDir::new("missing");
        let dict = load_dictionary(Some(&dir.0.join("SKK-JISYO.L")));
        assert!(dict.is_empty());
    }

    #[test]
    fn first_load_writes_a_cache_that_later_loads_reuse() {
        let dir = TempDir::new("writes-cache");
        let source = write_source_dictionary(&dir.0);

        let dict = load_dictionary(Some(&source));
        assert_eq!(dict.len(), 2);

        let cache = std::fs::read(compiled_cache_path(&source)).expect("cache written");
        assert_eq!(cache, dict.compile());
        let cached = load_compiled_cache(&compiled_cache_path(&source), &source)
            .expect("fresh cache is used");
        assert_eq!(cached.len(), 2);
    }

    #[test]
    fn corrupt_cache_is_rebuilt_from_source() {
        let dir = TempDir::new("corrupt-cache");
        let source = write_source_dictionary(&dir.0);
        let cache_path = compiled_cache_path(&source);
        std::fs::write(&cache_path, b"not a compiled dictionary").expect("write cache");

        let dict = load_dictionary(Some(&source));
        assert_eq!(dict.len(), 2);
        assert_eq!(std::fs::read(&cache_path).expect("cache"), dict.compile());
    }
}
