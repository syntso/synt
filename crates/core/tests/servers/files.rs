use data_encoding::HEXLOWER;
use serde_json::json;
use sha2::{Digest, Sha256};
use synt_client::Transferred;
use synt_proto::{ActionCall, ActionOutcome};

use crate::Lab;
use crate::lab::INNER;

const SIZE: usize = 1536 * 1024 + 9;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn an_upload_and_its_download_agree_on_sha256_with_the_journal() {
    let Some(lab) = Lab::start().await else {
        return;
    };
    let data = random_bytes(SIZE);
    let path = format!("/tmp/{}.bin", HEXLOWER.encode(&random_bytes(8)));
    let expected = HEXLOWER.encode(&Sha256::digest(&data));

    let (uploaded, sent) = upload(&lab, &path, &data).await;
    let (downloaded, received, bytes) = download(&lab, &path).await;
    let upload_journal = lab.finished(uploaded.request).await;
    let download_journal = lab.finished(downloaded.request).await;

    assert_eq!(sent.sha256, expected);
    assert_eq!(uploaded.value["sha256"], json!(expected));
    assert_eq!(upload_journal["sha256"], json!(expected));
    assert_eq!(upload_journal["bytes"], json!(SIZE));

    assert_eq!(bytes, data);
    assert_eq!(received.sha256, expected);
    assert_eq!(download_journal["sha256"], json!(expected));
    assert_eq!(download_journal["eof"], json!(true));

    lab.stop().await;
}

async fn upload(lab: &Lab, path: &str, data: &[u8]) -> (ActionOutcome, Transferred) {
    let mut stream = lab.client.open_stream();
    let size = data.len() as u64;
    let call = ActionCall::new("sftp.upload").target(INNER).params(json!({
        "path": path,
        "stream": stream.stream().get(),
        "size": size,
    }));

    let calling = lab.client.call(call);
    let sending = lab.client.send_credited(&mut stream, data, size);
    let (called, sent) = tokio::join!(calling, sending);

    (called.unwrap(), sent.unwrap())
}

async fn download(lab: &Lab, path: &str) -> (ActionOutcome, Transferred, Vec<u8>) {
    let mut stream = lab.client.open_stream();
    let mut bytes = Vec::new();
    let call = ActionCall::new("sftp.download")
        .target(INNER)
        .params(json!({ "path": path, "stream": stream.stream().get() }));

    let calling = lab.client.call(call);
    let receiving = lab.client.receive_credited(&mut stream, &mut bytes);
    let (called, received) = tokio::join!(calling, receiving);

    (called.unwrap(), received.unwrap(), bytes)
}

fn random_bytes(size: usize) -> Vec<u8> {
    let mut data = vec![0; size];
    getrandom::fill(&mut data).unwrap();

    data
}
