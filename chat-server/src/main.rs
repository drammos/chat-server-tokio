use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::broadcast,
};

#[derive(Clone)]
pub struct Users {
    users: HashMap<String, SocketAddr>,
}

impl Users {
    fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }

    fn check_the_name(&self, name: String) -> bool {
        if self.users.contains_key(&name) {
            return false;
        }
        true
    }

    fn add_user(&mut self, name: String, addr: SocketAddr) {
        self.users.insert(name, addr);
    }
}

fn check_out(line: String, a: Arc<Mutex<Users>>) -> bool {
    let map = a.lock().unwrap();
    map.check_the_name(line)
}

fn add_user_in_map(line: String, addr: SocketAddr, a: Arc<Mutex<Users>>) {
    let mut map = a.lock().unwrap();

    map.add_user(line, addr);
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let addr = "0.0.0.0:3000".to_string();

    let listener = TcpListener::bind(&addr).await.unwrap();

    let (tx, _rx) = broadcast::channel(100);

    let users = Arc::new(Mutex::new(Users::new()));

    loop {
        let (mut socket, address) = listener.accept().await.unwrap();

        let tx = tx.clone();
        let mut rx = tx.subscribe();
        let a = users.clone();
        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();

            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            //Read Name
            writer
                .write_all("Give Your name: ".as_bytes())
                .await
                .unwrap();
            let mut find = true;
            while find {
                tokio::select! {
                    result = reader.read_line(&mut line) => {
                        if result.unwrap() == 0{
                            break;
                        }
                        find = false;
                    }
                }
            }

            //check the name if is unique
            let mut result = check_out(line.clone(), a.clone());

            while !result {
                line.clear();

                writer
                    .write_all("Please Give Your name: ".as_bytes())
                    .await
                    .unwrap();
                let mut find = true;
                while find {
                    tokio::select! {
                        result = reader.read_line(&mut line) => {
                            if result.unwrap() == 0{
                                break;
                            }
                            find = false;
                        }
                    }
                }
                result = check_out(line.clone(), a.clone());
            }

            add_user_in_map(line.clone(), address, a);

            let mut user_name = line.clone();
            user_name = user_name.as_str().trim().to_string();
            line.clear();

            //Join in Chat
            writer
                .write_all("You join in Chat!\n".as_bytes())
                .await
                .unwrap();

            //Message in others users
            let hellomessage = format!("{} joinning in Chat!\n", user_name);
            tx.send((hellomessage, address)).unwrap();

            //chat
            loop {
                tokio::select! {
                    result = reader.read_line(&mut line) => {
                        if result.unwrap() == 0{
                            break;
                        }

                        let message = format!("{}: {}", user_name, line.clone());
                        tx.send((message, address)).unwrap();
                        line.clear();
                    }
                    result = rx.recv() => {
                        let (message, other_address) = result.unwrap();

                        if address != other_address{
                            writer.write_all( message.as_bytes()).await.unwrap();
                        }
                    }
                }
            }
        });
    }
}
