use futures::StreamExt;
use serial2::SerialPort;
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::{collections::HashMap, sync::Arc, sync::Mutex};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use warp::Filter;
use std::{io, thread};
use std::time::Duration;
pub mod models;
use models::InformationStruct;
static NEXT_USERID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;

impl InformationStruct {
    // Constructor to create an instance of InformationStruct from a text string
    pub fn from_text(text: &str) -> Self {
        let parts: Vec<&str> = text
            .trim()  // Remove leading and trailing whitespaces
            .split(',')  // Split the text by comma delimiter
            .map(|part| part.trim())  // Trim each part
            .collect();

        // Ensure that we have at least the expected number of parts
        // assert!(parts.len() >= 8, "Not enough parts in the text");
        if parts[0].to_string() == String::from("JI"){
            InformationStruct {
                command: parts[0].to_string(),
                serial_number: parts[1].to_string(),
                extended_data_1: parts[2].to_string(),
                extended_data_2: parts[3].to_string(),
                extended_data_3: parts[4].to_string(),
                subscription_expire_date: parts[5].to_string(),
                configuration_code: parts[6].to_string(),
                firmware_version_number: parts[7].to_string(),
            }
        }
        else if parts[0].to_string() == String::from("GPGGA") {
            InformationStruct {
                command: parts[0].to_string(),
                serial_number: parts[1].to_string(),
                extended_data_1: parts[2].to_string(),
                extended_data_2: parts[3].to_string(),
                extended_data_3: parts[4].to_string(),
                subscription_expire_date: parts[5].to_string(),
                configuration_code: parts[6].to_string(),
                firmware_version_number: parts[7].to_string(),
            }
        }   
        else{
            println!("Error!");
            InformationStruct {
                command: parts[0].to_string(),
                serial_number: parts[1].to_string(),
                extended_data_1: parts[2].to_string(),
                extended_data_2: parts[3].to_string(),
                extended_data_3: parts[4].to_string(),
                subscription_expire_date: parts[5].to_string(),
                configuration_code: parts[6].to_string(),
                firmware_version_number: parts[7].to_string(),
            }
            }
        }
    }

#[tokio::main]
async fn main() {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());
    let socket_address: SocketAddr = addr.parse().expect("valid socket Address");
    // let mut port: Box<dyn SerialPort> = match serialport::new("/dev/ttyUSB0", 115200)
    // .timeout(std::time::Duration::from_secs(5))
    // .open()
    // {
    //     Ok(port) => port,
    //     Err(err) => panic!("Failed to open serial port: {}", err),
    // };
    let port_name = "/dev/ttyUSB0";
	let baud_rate: u32 = 115200;


	let port =match SerialPort::open(port_name, baud_rate)
		.map_err(|e| eprintln!("Error: Failed to open {}: {}", port_name, e)){
            Ok(port) => port,
            Err(err) => panic!("Failed to open serial port: {:?}",err),

            
        };
    let last_command: HashMap<usize, String> = HashMap::new();
    let last_command = Arc::new(Mutex::new(last_command));
    let last_commands = warp::any().map(move || last_command.clone());
	let port = Arc::new(port);
    let users = Users::default();
    let users = warp::any().map(move || users.clone());
    let ports = warp::any().map(move || port.clone());
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
        .and(ports)
        .and(last_commands)
        .map(|ws: warp::ws::Ws, users: Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>, ports: Arc<SerialPort>, last_commands: Arc<Mutex<HashMap<usize, String>>>| ws.on_upgrade( move |socket| connect(socket, users, ports,last_commands)));

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

async fn connect(ws: WebSocket, users: Users, port: Arc<SerialPort>,last_command:Arc<Mutex<HashMap<usize, String>>> ) {
    // Bookkeeping
    let my_id = NEXT_USERID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    println!("Welcome User {}", my_id);
    // Establishing a connection
    let (user_tx, mut user_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);

    tokio::spawn(rx.forward(user_tx));
    users.write().await.insert(my_id, tx.clone());
         
    let clone: Arc<SerialPort> = Arc::clone(&port);
    
    
    let last = Arc::clone(&last_command);
    thread::spawn(move ||{
        let mut concat_once = false;
        let mut remained_message = String::new();
        loop {
            thread::sleep(Duration::from_millis(500));
            
            let mut serial_buf: Vec<u8> = vec![0; 512];
            match clone.read(serial_buf.as_mut_slice()) {
                Ok(bytes_read) => {
                    let mut received_string = String::from_utf8_lossy(&serial_buf[..bytes_read]).to_string();
                    println!("Received: {:?}", received_string);
                    // let infoS = InformationStruct::from_text(&received_string);
                    // let info = models::create_info(infoS);
                    // let bin = models::serialize(&info);
                    let last = last.lock().unwrap();
                    let res = last.get(&my_id.clone());
                    if let Some(command) = res {
                        loop{
                        if received_string.contains(command){
                            if command == &String::from("JI"){
                                    // Find the starting index of "$>JI"
                                let start_index = match received_string.find("JI") {
                                    Some(idx) => idx,
                                    None => {
                                        println!("No '$>JI' found in the input string.");
                                        continue;
                                    }
                                };
                                
                                // Find the ending index of "\r\n" after the "$>JI"
                                let end_index = match received_string[start_index..].find("\r\n") {
                                    Some(idx) => {
                                        remained_message = received_string[start_index + idx + 2..].to_string();
                                        start_index + idx + 2 // Add 2 to include "\r\n" in the substring
                                    }
                                    None => {
                                        remained_message = received_string[start_index..].to_string();
                                        println!("No '\\r\\n' found after '$>JI' in the input string. and remained message set to {}",remained_message);
                                        break;
                                    }
                                };
                                println!("user {} last command is {}",my_id,command);
                                tx.send(Ok(Message::text(&mut received_string[start_index..end_index]).clone())).expect("Failed to send message");
                                concat_once = false;
                                break;
                            }
                            if command == &String::from("GPGGA"){
                                // Find the starting index of "$>JI"
                            let start_index = match received_string.find("GPGGA") {
                                Some(idx) => idx,
                                None => {
                                    println!("No '$>GPGGA' found in the input string.");
                                    continue;
                                }
                            };
                            
                            // Find the ending index of "\r\n" after the "$>JI"
                            let end_index = match received_string[start_index..].find("\r\n") {
                                Some(idx) => {
                                    remained_message = received_string[start_index + idx + 2..].to_string();
                                    start_index + idx + 2 // Add 2 to include "\r\n" in the substring
                                    
                                }                                None => {
                                    remained_message = received_string[start_index..].to_string();
                                    println!("No '\\r\\n' found after '$>JI' in the input string. and remained message set to {}",remained_message);
                                    break;
                                }
                            };
                            println!("user {} last command is {}",my_id,command);

                            tx.send(Ok(Message::text(&mut received_string[start_index..end_index]).clone())).expect("Failed to send message");
                            concat_once = false;
                            break;

                        }
                        }
                        else if concat_once == false && !remained_message.is_empty(){
                            received_string = remained_message.clone() + &received_string;
                            println!("the remained message is {} and the new messsage is {}", remained_message, received_string);
                            concat_once = true;
                        }
                        else if remained_message.is_empty(){
                            remained_message = received_string.clone();
                            println!("the remained message is {} and the new messsage is {}", remained_message, received_string);
                            break;
                        }
                        else{
                            break;
                        }
                    }
                    } else {
                        println!("Key not found in the map");
                      }
                    
                    

                    
                }
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                Err(e) => eprintln!("{:?}", e),
            }
        }
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
                if text.contains("JI")
                {
                    last_command.clone().lock().unwrap().insert(my_id, String::from("JI"));
                    for (key, value) in last_command.clone().lock().unwrap().iter() {
                        println!(" Key: {}, Value: {}", key, value);
                      }
                }
                if text.contains("JASC")
                {
                    last_command.clone().lock().unwrap().insert(my_id, String::from("GPGGA"));
                }
                
            },
            Err(err) => {
                eprintln!("Error converting bytes to string: {}", err);
            }
        }
    }
    

    disconnect(my_id, &users).await;
}

async fn disconnect(my_id: usize, users: &Users) {
    println!("Good bye user {}", my_id);

    users.write().await.remove(&my_id);
}
