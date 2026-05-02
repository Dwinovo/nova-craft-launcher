use ncl_core::errors::{Error, Result};
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// 计算文件 SHA1，返回小写 hex。
pub fn sha1_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).map_err(|e| Error::io(path, e))?;
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).map_err(|e| Error::io(path, e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn verify_sha1(path: &Path, expected: &str) -> Result<()> {
    let actual = sha1_file(path)?;
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(Error::ChecksumMismatch {
            expected: expected.to_string(),
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn known_sha1_of_abc() {
        let f = tempfile_path();
        std::fs::File::create(&f).unwrap().write_all(b"abc").unwrap();
        assert_eq!(
            sha1_file(&f).unwrap(),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        let _ = std::fs::remove_file(f);
    }

    fn tempfile_path() -> std::path::PathBuf {
        let dir = std::env::temp_dir();
        dir.join(format!("ncl-net-test-{}", std::process::id()))
    }
}
