use std::{collections::HashMap, sync::{Arc, Mutex}};
use bytes::Bytes;
use mini_redis::{Connection, Frame};
use tokio::net::{TcpListener, TcpStream};

type Db = Arc<Mutex<HashMap<String, Bytes>>>;

#[tokio::main]
async fn main() {
    // リスナーをこのアドレスにバインドする
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    println!("Listening");

    let db = Arc::new(Mutex::new(HashMap::new()));

    loop {
        // タプルの2つ目の要素は, 新しいコネクションのIPとポートの情報を含んでいる
        let (socket, _) = listener.accept().await.unwrap();

        // ハッシュマップへのハンドルを複製
        let db = db.clone();

        println!("Accepted");
        // それぞれのインバウンドソケットに対して新しいタスクをspawnする
        // ソケットは新しいタスクにmoveされ, そこで処理される.
        tokio::spawn(async move {
            process(socket, db).await;
        });
    }
}

async fn process(socket: TcpStream, db: Db) {
    use mini_redis::Command::{self, Get, Set};

    // 'Connection'を使うことで, バイト列ではなく,
    // Redisのフレームを読み書きすることができるようになる.
    // この型はmini-redisで定義されている
    let mut connection = Connection::new(socket);

    if let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                let mut db = db.lock().unwrap();
                // Vec<u8> として保持する
                db.insert(cmd.key().to_string(), cmd.value().clone());
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                let db = db.lock().unwrap();
                if let Some(value) = db.get(cmd.key()) {
                    // Frame::Bulk はデータがBytes' 型であることを期待する
                    Frame::Bulk(value.clone().into())
                } else {
                    Frame::Null
                }
            }
            cmd => panic!("unimplemented {:?}", cmd),
        };

        connection.write_frame(&response).await.unwrap();
    }
}
