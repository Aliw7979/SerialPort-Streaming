use futures::StreamExt;
use serialport::SerialPort;
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use warp::Filter;
use std::io::Write;
use std::{io, thread};
use std::time::Duration;


static NEXT_USERID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;

#[tokio::main]
async fn main() {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());
    let socket_address: SocketAddr = addr.parse().expect("valid socket Address");

    let users = Users::default();
    let users = warp::any().map(move || users.clone());

    let opt = warp::path::param::<String>()
        .map(Some)
        .or_else(|_| async { Ok::<(Option<String>,), std::convert::Infallible>((None,)) });

    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let hello = warp::path("hello")
        .and(opt)
        .and(warp::path::end())
        .map(|name: Option<String>| {
            format!("Hello, {}!", name.unwrap_or_else(|| "world".to_string()))
        });

    // GET /ws
    let chat = warp::path("ws")
        .and(warp::ws())
        .and(users)
        .map(|ws: warp::ws::Ws, users| ws.on_upgrade(move |socket| connect(socket, users)));

    let files = warp::fs::dir("./static");

    let res_404 = warp::any().map(|| {
        warp::http::Response::builder()
            .status(warp::http::StatusCode::NOT_FOUND)
            .body(fs::read_to_string("./static/404.html").expect("404 404?"))
    });

    let routes = chat.or(hello).or(files).or(res_404);

    let server = warp::serve(routes).try_bind(socket_address);

    println!("Running server at {}!", addr);

    server.await
}

async fn connect(ws: WebSocket, users: Users) {
    // Bookkeeping
    let my_id = NEXT_USERID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    println!("Welcome User {}", my_id);

    // Establishing a connection
    let (user_tx, mut user_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel();

    let rx = UnboundedReceiverStream::new(rx);

    tokio::spawn(rx.forward(user_tx));
    users.write().await.insert(my_id, tx.clone());
            let mut port = match serialport::new("/dev/ttyUSB0", 115200)
            .timeout(std::time::Duration::from_secs(5))
            .open()
        {
            Ok(port) => port,
        Err(err) => panic!("Failed to open serial port: {}", err),
    };
    
    let mut clone = port.try_clone().expect("Failed to clone");
    let mut serial_buf: Vec<u8> = vec![0; 100];
    
    thread::spawn(move ||
        loop {
            match clone.read(serial_buf.as_mut_slice()) {
                Ok(bytes_read) => {
                    let received_string = String::from_utf8_lossy(&serial_buf[..bytes_read]).to_string();
                    println!("Received: {:?}", received_string);
                    tx.send(Ok(Message::text(received_string).clone())).expect("Failed to send message");
                    
                }
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                Err(e) => eprintln!("{:?}", e),
            }
            thread::sleep(Duration::from_millis(1000));
        }
    );
    
    while let Some(result) = user_rx.next().await {

        match String::from_utf8(<Message as Clone>::clone(&result.as_ref().expect("failed to fetch message.")).into_bytes()) {
            Ok(text) => {
                let byt = result.as_ref().expect("sherover").to_str().unwrap().as_bytes();

                let newline = [13, 10]; 
        
                let mut new_data = Vec::from(byt); 
                new_data.insert(0, 36); 
                new_data.extend(newline.iter()); 
                let new_slice = &new_data[..];        
                println!("{}", text);
                if let Err(err) = port.write_all(new_slice) {
                    panic!("Failed to write data to serial port: {}", err);
                }
            },
            Err(err) => {
                eprintln!("Error converting bytes to string: {}", err);
            }
        }
    }
    

    disconnect(my_id, &users).await;
}

async fn broadcast_msg(msg: Message, users: &Users) {
    if let Ok(_) = msg.to_str() {
        for (&_uid, tx) in users.read().await.iter() {
            tx.send(Ok(msg.clone())).expect("Failed to send message");
        }
    }
}
async fn send_data_to_serial_port_and_get_response(msg: Message,port: &mut Box<dyn SerialPort>) {    
    if let Err(err) = port.write_all(&msg.as_bytes()) {
        panic!("Failed to write data to serial port: {}", err);
    }
}

async fn disconnect(my_id: usize, users: &Users) {
    println!("Good bye user {}", my_id);

    users.write().await.remove(&my_id);
}
