use crate::errors::Result;
use minifier::js::minify;
use sha3::{Digest, Sha3_256};
use std::path::{Path, PathBuf};

#[derive(Clone)]

/// This special wrapper used to verify the contents of JS files.
///
/// This struct is responsible for reading JS file from ./assets/js/*
/// and verifying their contents match their hash when returned
pub struct JsFile {
    hash: String,
    content: String,
}

impl JsFile {
    pub fn new(filename: &str) -> Result<JsFile> {
        let dir = PathBuf::new().join("./src/assets/js");
        let path = dir.clone().join(format!("{filename}.js"));

        // Make SURE the file is within the ./assets/js directory
        if !is_within_directory(&dir, &path) {
            let kind = std::io::ErrorKind::NotFound;
            Err(std::io::Error::new(
                kind,
                "File is not within the specified directory",
            ))?;
        }

        let content = std::fs::read_to_string(path)?;

        let mut hasher = Sha3_256::new();
        hasher.update(content.as_bytes());
        let hash = hasher.finalize();
        let hex_hash = base16ct::lower::encode_string(&hash);
        Ok(JsFile {
            hash: hex_hash,
            content,
        })
    }

    pub fn contents(&self) -> &str {
        &self.content
    }

    pub fn min_contents(self) -> String {
        minify(&self.content).to_string()
    }

    pub fn verify_hash(&self, hash: &str) -> Result<&Self> {
        if !self.hash.starts_with(hash) || hash.is_empty() {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "File Hash Invalid",
            ))?;
        }
        Ok(self)
    }
}

fn is_within_directory(dir: &Path, filename: &Path) -> bool {
    // Get the absolute canonical paths of both `dir` and `filename`
    if let (Ok(dir_abs), Ok(filename_abs)) = (dir.canonicalize(), filename.canonicalize()) {
        // Check if `filename_abs` starts with `dir_abs`, meaning it's inside the directory
        return filename_abs.starts_with(&dir_abs);
    }
    false
}

/// Returns the URL to the request javascript file.
/// This URL contains the JS file's hashed,
/// so when JS files change, new versions are served up
pub fn js_path(filename: &str) -> Result<String> {
    let file = JsFile::new(filename)?;
    let hash = &file.hash[0..10];
    Ok(format!("/assets/js/{filename}-{hash}.js"))
}
