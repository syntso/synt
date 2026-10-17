use std::time::Duration;

use bytes::Bytes;
use serde_json::{Value, json};
use synt_client::{StreamEvent, StreamReceiver};
use synt_proto::{ActionCall, EndReason, SessionEnded, StreamId};
use synt_testkit::TestError;
use tokio::time;

use crate::Lab;
use crate::lab::INNER;

const LIMIT: Duration = Duration::from_secs(20);
const MARKED: &str = "printf '\\033]133;A\\007$ \\033]133;B\\007'; \
                      printf '\\033]133;C;cmdline=echo inside\\007'; echo inside; \
                      printf '\\033]133;D;5\\007'\n";

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_terminal_session_turns_osc_133_marks_into_journaled_blocks() -> Result<(), TestError> {
    let Some(lab) = Lab::start().await? else {
        return Ok(());
    };

    let opened = lab
        .call(
            INNER,
            "ssh.session.open",
            json!({ "cols": 100, "rows": 30 }),
        )
        .await
        .unwrap();
    let stream: StreamId = serde_json::from_value(opened.value["stream"].clone()).unwrap();
    let session = opened.value["session"].clone();
    let mut output = lab.client.stream(stream);

    send(&lab, stream, MARKED).await?;
    let blocks = time::timeout(LIMIT, blocks_of(&lab, &session)).await??;

    send(&lab, stream, "exit 7\n").await?;
    let ended = time::timeout(LIMIT, ending(&mut output)).await.unwrap();
    let finished = lab.finished(opened.request).await?;

    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0]["command"], json!("echo inside"));
    assert_eq!(blocks[0]["exit"], json!(5));
    assert_eq!(ended.reason, EndReason::Exited);
    assert_eq!(ended.exit, Some(7));
    assert_eq!(finished["blocks"], json!(1));
    assert_eq!(finished["exit"], json!(7));
    assert_eq!(finished["reason"], json!("exited"));

    lab.stop().await?;

    Ok(())
}

async fn send(lab: &Lab, stream: StreamId, text: &str) -> Result<(), TestError> {
    lab.client
        .send_stream(stream, Bytes::copy_from_slice(text.as_bytes()))
        .await?;

    Ok(())
}

async fn blocks_of(lab: &Lab, session: &Value) -> Result<Vec<Value>, TestError> {
    loop {
        let call = ActionCall::new("session.blocks").params(json!({ "session": session }));
        let listed = lab.client.call(call).await?;
        let blocks = listed
            .value
            .as_array()
            .ok_or("session.blocks gave no list")?
            .clone();

        if !blocks.is_empty() {
            return Ok(blocks);
        }

        time::sleep(Duration::from_millis(50)).await;
    }
}

async fn ending(output: &mut StreamReceiver) -> SessionEnded {
    loop {
        match output.next().await {
            Some(StreamEvent::Data(_)) => {}
            Some(StreamEvent::Ended(ended)) => return ended,
            None => panic!("the connection closed before the session ended"),
        }
    }
}
