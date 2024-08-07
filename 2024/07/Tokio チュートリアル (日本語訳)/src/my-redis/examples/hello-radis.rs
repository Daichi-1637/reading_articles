use bytes::Bytes;
use mini_redis::{client, Result};
use tokio::sync::{mpsc, oneshot};

type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;

#[derive(Debug)]
enum Command {
    Get { 
        key: String,
        resp: Responder<Option<Bytes>>,
    },
    Set { 
        key: String, 
        val: Vec<u8>, 
        resp: Responder<()>, 
    },
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let (tx, mut rx) = mpsc::channel(32);
    let tx2 = tx.clone();

    let manager = tokio::spawn(async move {
        // mini-redis アドレスへのコネクションを開く
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();

        while let Some(cmd) = rx.recv().await {
            match cmd {
                Command::Get { key, resp } => {
                    let res = client.get(&key).await;
                    let _ = resp.send(res);
                }
                Command::Set { key, val, resp } => {
                    let res = client.set(&key, val.into()).await;
                    let _ = resp.send(res);
                }
            }
        }
    });

    let t1 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Get {
            key: "hello".to_string(),
            // Command にoneshotのチェネルのSenderを持たせておくことで, 
            // このチェネルを介してmanagerからレスポンスが返ってくる
            resp: resp_tx
        };

        // Getリクエストを送信
        tx.send(cmd).await.unwrap();

        // レスポンスが来るのを待つ
        let res = resp_rx.await;
        print!("GOT = {:?}", res);
    });

    let t2 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Set {
            key: "foo".to_string(),
            val: b"bar".to_vec(),
            resp: resp_tx
        };

        // SET リクエストを送信
        tx2.send(cmd).await.unwrap();

        // レスポンスが来るのを待つ
        let res = resp_rx.await;
        println!("GOT = {:?}", res);
    });


    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();

    Ok(())
}
