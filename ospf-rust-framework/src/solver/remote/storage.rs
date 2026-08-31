//! 远程求解对象存储实现
//! Remote solver object storage implementations

use super::domain::{
    ObjectEtag, ObjectPath, ObjectRef, ObjectVersion, RemoteSolverError, RemoteSolverErrorCode,
    RemoteSolverResult,
};
use super::port::ObjectStoragePort;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 本地文件对象存储。
/// Local file object storage.
#[derive(Debug, Clone)]
pub struct LocalFileObjectStoragePort {
    root: PathBuf,
}

impl LocalFileObjectStoragePort {
    /// 创建本地文件对象存储。
    /// Create a local file object storage.
    pub fn try_new(root: impl Into<PathBuf>) -> RemoteSolverResult<Self> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(storage_io_error)?;
        let root = root.canonicalize().map_err(storage_io_error)?;
        Ok(Self { root })
    }

    /// 获取根目录。
    /// Get storage root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn resolve_path(&self, path: &ObjectPath) -> RemoteSolverResult<PathBuf> {
        let mut relative = PathBuf::new();
        for component in Path::new(path.value()).components() {
            match component {
                Component::Normal(segment) => relative.push(segment),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(RemoteSolverError::invalid_argument(format!(
                        "Object path '{}' escapes storage root.",
                        path
                    )));
                }
            }
        }
        if relative.as_os_str().is_empty() {
            return Err(RemoteSolverError::invalid_argument(
                "Object path must resolve to a file.",
            ));
        }

        let resolved = self.root.join(relative);
        if !resolved.starts_with(&self.root) {
            return Err(RemoteSolverError::invalid_argument(format!(
                "Object path '{}' escapes storage root.",
                path
            )));
        }
        Ok(resolved)
    }

    fn resolve_ref_path(&self, object_ref: &ObjectRef) -> RemoteSolverResult<PathBuf> {
        self.resolve_path(&object_ref.path)
    }

    fn metadata_path(path: &Path) -> RemoteSolverResult<PathBuf> {
        let file_name = path
            .file_name()
            .and_then(|item| item.to_str())
            .ok_or_else(|| {
                RemoteSolverError::invalid_argument("Object path must contain a valid file name.")
            })?;
        Ok(path.with_file_name(format!("{}.metadata", file_name)))
    }
}

#[async_trait]
impl ObjectStoragePort for LocalFileObjectStoragePort {
    async fn put(
        &self,
        path: &ObjectPath,
        bytes: &[u8],
        metadata: &BTreeMap<String, String>,
    ) -> RemoteSolverResult<ObjectRef> {
        let resolved = self.resolve_path(path)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).map_err(storage_io_error)?;
        }

        fs::write(&resolved, bytes).map_err(storage_io_error)?;
        let etag = ObjectEtag::of(sha256_hex(bytes))?;
        let version = ObjectVersion::of(current_epoch_millis().to_string())?;
        let sidecar = LocalFileObjectMetadata {
            metadata: metadata.clone(),
            etag: etag.value().to_string(),
            version: version.value().to_string(),
            content_length: bytes.len(),
        };
        let sidecar_bytes = serde_json::to_vec_pretty(&sidecar).map_err(|err| {
            RemoteSolverError::new(
                RemoteSolverErrorCode::StorageIoFailed,
                format!("Failed to serialize object metadata: {}", err),
            )
        })?;
        fs::write(Self::metadata_path(&resolved)?, sidecar_bytes).map_err(storage_io_error)?;

        Ok(ObjectRef::new(path.clone())
            .with_etag(etag)
            .with_version(version))
    }

    async fn get(&self, object_ref: &ObjectRef) -> RemoteSolverResult<Option<Vec<u8>>> {
        let resolved = self.resolve_ref_path(object_ref)?;
        match fs::read(resolved) {
            Ok(bytes) => {
                validate_object_ref_etag(object_ref, &bytes)?;
                Ok(Some(bytes))
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(storage_io_error(err)),
        }
    }

    async fn delete(&self, object_ref: &ObjectRef) -> RemoteSolverResult<bool> {
        let resolved = self.resolve_ref_path(object_ref)?;
        let existed = resolved.exists();
        if existed {
            fs::remove_file(&resolved).map_err(storage_io_error)?;
        }
        let metadata_path = Self::metadata_path(&resolved)?;
        if metadata_path.exists() {
            fs::remove_file(metadata_path).map_err(storage_io_error)?;
        }
        Ok(existed)
    }

    async fn exists(&self, object_ref: &ObjectRef) -> RemoteSolverResult<bool> {
        Ok(self.resolve_ref_path(object_ref)?.exists())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalFileObjectMetadata {
    metadata: BTreeMap<String, String>,
    etag: String,
    version: String,
    content_length: usize,
}

fn storage_io_error(err: std::io::Error) -> RemoteSolverError {
    RemoteSolverError::new(
        RemoteSolverErrorCode::StorageIoFailed,
        format!("Local object storage I/O failed: {}", err),
    )
}

/// 校验对象引用携带的内容摘要 / Validate the content digest carried by an object reference.
pub(crate) fn validate_object_ref_etag(
    object_ref: &ObjectRef,
    bytes: &[u8],
) -> RemoteSolverResult<()> {
    if let Some(expected_etag) = object_ref.etag.as_ref()
        && expected_etag.value() != sha256_hex(bytes)
    {
        return Err(RemoteSolverError::invalid_argument(format!(
            "object '{}' failed ETag validation",
            object_ref.path
        )));
    }
    Ok(())
}

fn current_epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push_str(&format!("{:02x}", byte));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ospf-rust-remote-storage-{}-{}",
            name,
            current_epoch_millis()
        ))
    }

    #[tokio::test]
    async fn local_file_storage_put_get_exists_and_delete_object() {
        let root = temp_root("roundtrip");
        let storage = LocalFileObjectStoragePort::try_new(&root).unwrap();
        let path = ObjectPath::of("tenant-a/tasks/task-1/model.json").unwrap();
        let mut metadata = BTreeMap::new();
        metadata.insert("contentType".to_string(), "application/json".to_string());

        let object_ref = storage
            .put(&path, br#"{"ok":true}"#, &metadata)
            .await
            .unwrap();

        assert!(object_ref.etag.is_some());
        assert!(object_ref.version.is_some());
        assert!(storage.exists(&object_ref).await.unwrap());
        assert_eq!(
            storage.get(&object_ref).await.unwrap(),
            Some(br#"{"ok":true}"#.to_vec())
        );
        assert!(
            storage
                .root()
                .join("tenant-a/tasks/task-1/model.json.metadata")
                .exists()
        );
        assert!(storage.delete(&object_ref).await.unwrap());
        assert!(!storage.exists(&object_ref).await.unwrap());

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn local_file_storage_rejects_path_escape() {
        let root = temp_root("escape");
        let storage = LocalFileObjectStoragePort::try_new(&root).unwrap();
        let path = ObjectPath::of("../outside.bin").unwrap();

        let err = storage
            .put(&path, b"bad", &BTreeMap::new())
            .await
            .unwrap_err();

        assert_eq!(err.code, RemoteSolverErrorCode::InvalidArgument);
        let _ = fs::remove_dir_all(root);
    }
}
