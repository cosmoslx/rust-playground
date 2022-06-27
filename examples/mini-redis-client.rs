use mini_redis::{client, Result};
use bytes::Bytes;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

type Responder<T> = oneshot::Sender<Result<T>>;

#[derive(Debug)]
enum Command {
    Get {
        key: String,
        resp: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        val: Bytes,
        resp: Responder<()>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let (tx, mut rx) = mpsc::channel(32);
    let tx2 = tx.clone();

    let t1 = tokio::spawn(async move {
        println!("[t1] begin");
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Get {
            key: "hello".to_owned(),
            resp: resp_tx,
        };

        tx.send(cmd).await.unwrap();

        let resp = resp_rx.await.unwrap();
        println!("[t1] Got = {:?}", resp);
    });

    let t2 = tokio::spawn(async move {
        println!("[t2] begin");
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Set {
            key: "hello".to_owned(),
            val: "bar".into(),
            resp: resp_tx,
        };

        tx2.send(cmd).await.unwrap();

        let resp = resp_rx.await.unwrap();
        println!("[t2] Got = {:?}", resp);
    });

    let manager = tokio::spawn(async move {
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();

        while let Some(cmd) = rx.recv().await {
            match cmd {
                Command::Get { key, resp, } => {
                    let res = client.get(&key).await;
                    let _ = resp.send(res);
                },
                Command::Set { key, val, resp,} => {
                    let res = client.set(&key, val).await;
                    let _ = resp.send(res);
                },
            }
        }
    });

    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();

    Ok(())
}

#[allow(dead_code)]
async fn simple_main() -> Result<()> {
    let mut client = client::connect("127.0.0.1:6379").await?;

    client.set("hello", "world".into()).await?;

    let result = client.get("hello").await?;

    println!("result: {:?}", result);

    Ok(())
}

#[allow(dead_code)]
async fn send_recv_async() {
    let (tx, mut rx) = mpsc::channel(32);
    let tx2 = tx.clone();

    tokio::spawn(async move {
        tx.send("Sending from first").await.unwrap();
    });

    tokio::spawn(async move {
        tx2.send("Sending from second").await.unwrap();
    });

    while let Some(message) = rx.recv().await {
        println!("Got = {}", message);
    }
}
