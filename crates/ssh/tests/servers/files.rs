use std::sync::Arc;

use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};
use synt_ssh::{FileEntry, FileSessions, Sftp, SshError, WriteMode};

use crate::Setup;

const SIZE: usize = 3 * 1024 * 1024 + 517;
const CHUNK: usize = 100_000;

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn an_uploaded_file_downloads_back_with_the_same_sha256() {
    let Some(setup) = Setup::start() else { return };
    let target = setup.target();
    let files = FileSessions::new(setup.pool.clone());
    let path = format!("/tmp/{}.bin", unique_name());
    let data = random_bytes(SIZE);

    files
        .with(&target, |sftp| upload(sftp, &path, &data))
        .await
        .unwrap();
    let downloaded = files
        .with(&target, |sftp| download(sftp, &path))
        .await
        .unwrap();
    let on_server = setup
        .run(&target, &format!("sha256sum {path}"))
        .await
        .unwrap();

    assert_eq!(downloaded.len(), SIZE);
    assert_eq!(sha256(&downloaded), sha256(&data));
    assert!(on_server.stdout.starts_with(&sha256(&data)));
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_missing_file_is_reported_with_its_path() {
    let Some(setup) = Setup::start() else { return };
    let files = FileSessions::new(setup.pool.clone());
    let path = format!("/tmp/{}-missing", unique_name());

    let missing = files.with(&setup.bastion(), |sftp| stat(sftp, &path)).await;

    assert!(matches!(
        missing,
        Err(SshError::Sftp {
            operation: "stat",
            ..
        })
    ));
}

async fn stat(sftp: Arc<Sftp>, path: &str) -> Result<FileEntry, SshError> {
    sftp.stat(path).await
}

async fn upload(sftp: Arc<Sftp>, path: &str, data: &[u8]) -> Result<(), SshError> {
    let mode = WriteMode {
        create: true,
        truncate: true,
    };
    let mut upload = sftp.upload(path, 0, mode, true).await?;

    for chunk in data.chunks(CHUNK) {
        upload.write(chunk).await?;
    }

    upload.finish().await
}

async fn download(sftp: Arc<Sftp>, path: &str) -> Result<Vec<u8>, SshError> {
    let mut download = sftp.download(path, 0, None).await?;
    let mut data = Vec::new();

    while let Some(piece) = download.next().await? {
        data.extend_from_slice(&piece);
    }

    Ok(data)
}

fn random_bytes(size: usize) -> Vec<u8> {
    let mut data = vec![0; size];
    getrandom::fill(&mut data).unwrap();

    data
}

fn unique_name() -> String {
    HEXLOWER.encode(&random_bytes(8))
}

fn sha256(data: &[u8]) -> String {
    HEXLOWER.encode(&Sha256::digest(data))
}
