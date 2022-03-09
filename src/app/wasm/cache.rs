use std::fs;
use std::fs::File;
use std::io::ErrorKind;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::time::SystemTime;
use anyhow::{Result, anyhow};
use itertools::Itertools;

pub trait ModuleCache {
    fn get(&self, image: &str) -> Result<Option<Vec<u8>>>;
    fn put(&self, image: &str, data: &[u8]) -> Result<()>;
    fn purge(&self, max_size_mib: u64) -> Result<()>;
}

pub fn new_fs_cache(base_dir: PathBuf) -> FSCache {
    FSCache {
        base_dir,
    }
}

#[derive(Debug)]
pub struct FSCache {
    base_dir: PathBuf
}

impl FSCache {
    fn canonical_name(image: &str) -> String {
        image.chars().map(|c| match c {
            '/' => '-',
            ':' => '-',
            _ => c,
        }).collect()
    }
}

impl ModuleCache for FSCache {
    // TODO Add concurrency control or switch to using wasmtime-cache crate

    #[tracing::instrument(name = "fscache.get")]
    fn get(&self, image: &str) -> Result<Option<Vec<u8>>> {
        let image = FSCache::canonical_name(image);
        let path = self.base_dir.join(image);
        let f = match File::open(&path) {
            Ok(f) => f,
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    tracing::debug!("Cache miss for {:?}", path);
                    return Ok(None)
                }
                return Err(err.into());
            },
        };
        tracing::debug!("Cache hit for {:?}", path);
        let buf = zstd::stream::decode_all(f)?;
        Ok(Some(buf))
    }

    #[tracing::instrument(name = "fscache.put")]
    fn put(&self, image: &str, data: &[u8]) -> Result<()> {
        let image = FSCache::canonical_name(image);
        let path = self.base_dir.join(image);
        let f = File::create(&path)?;
        let _ = zstd::stream::copy_encode(data, f, 0)?;
        Ok(())
    }

    // TODO Add tokio task for running cache purge
    #[tracing::instrument(name = "fscache.purge")]
    fn purge(&self, max_size_mib: u64) -> Result<()> {
        let paths = fs::read_dir(&self.base_dir)?;
        // paths.collect::<Result<Vec<_>, _>>()?
        //     .iter():
        let files  = paths
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .map(|path| {
                let metadata = match path.metadata() {
                    Ok(meta) => meta,
                    Err(err) => return Err(anyhow!(err).context(format!("Reading file \"{:?}\" failed", path)))
                };
                let modified = match metadata.modified() {
                    Ok(modified) => modified,
                    Err(err) => return Err(anyhow!(err).context(format!("Reading mtime of file \"{:?}\" failed", path)))
                };
                Ok(CachedFile {
                    path: path.path(),
                    size_mib: metadata.size() / 1024 / 1024,
                    modified_at: modified,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let total_size: u64 = files.iter().map(|f| f.size_mib).sum();
        if total_size < max_size_mib {
            tracing::debug!("Not purging cache, total cached files: {}MiB, max size: {}MiB", total_size, max_size_mib);
            return Ok(())
        }

        let mut deleted_mib: u64 = 0;
        for (i, file) in files.
            into_iter()
            .sorted_by(|a,b| a.modified_at.cmp(&b.modified_at))
            .enumerate() {
            match fs::remove_file(&file.path) {
                Ok(_) => (),
                Err(err) => return Err(anyhow!(err).context(format!("Deleting file \"{:?}\" failed", &file.path)))
            }
            deleted_mib += file.size_mib;
            if total_size - deleted_mib <= max_size_mib {
                tracing::info!("Cached purged, deleted {}MiB in {} files, now using {}MiB out of {}MiB", deleted_mib, i, total_size - deleted_mib, max_size_mib);
            } else {
                tracing::debug!("Deleted file #{} ({:?}), continuing to delete", i, &file.path)
            }
        }

        Ok(())
    }
}

struct CachedFile {
    path: PathBuf,
    size_mib: u64,
    modified_at: SystemTime,
}

pub fn new_nop_cache() -> NopCache {
    NopCache {}
}

pub struct NopCache {}

impl ModuleCache for NopCache {
    fn get(&self, _image: &str) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    fn put(&self, _image: &str, _data: &[u8]) -> Result<()> {
        Ok(())
    }

    fn purge(&self, _max_size_mib: u64) -> Result<()> {
        Ok(())
    }
}
