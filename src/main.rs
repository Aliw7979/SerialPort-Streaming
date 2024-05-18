extern crate websocket;
// Importing necessary modules from the Rust libraries
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::RwLock;
use std::sync::Arc;
use serialport;
use websocket::ws::dataframe::DataFrame;
use std::{io, thread};
extern crate env_logger;
extern crate ws;
type Buff = Arc<RwLock<String>>;
// use ws::{connect, listen, CloseCode, Handler, Message, Result, Sender};
use std::fs::File;
use std::io::Result as res;
use websocket::sync::Server;
use websocket::OwnedMessage;

#[derive(Debug, Default,Clone)]
struct FormData {
    running : bool,
    session_name: String,
    file_len : String,
}

// Define your struct containing Arc<RwLock<T>>

#[derive(Debug, Default,Clone)]
struct SharedData {
    data: Arc<RwLock<FormData>>,
}


fn handle_client(mut stream: TcpStream,buffer : Buff){
    // this is a buffer to read data from the client
    // let mut buffer = [0; 1024];
    // this line reads data from the stream and stores it in the buffer.
    // stream.read(&mut buffer).expect("Failed to read from client!");
    // this line converts the data in the buffer into a UTF-8 enccoded string.
    // let request = String::from_utf8_lossy(&buffer[..]);
    // println!("Received request: {}", request);
    loop{
        thread::sleep(std::time::Duration::from_secs(1));

        let buff_to_read = buffer.read().unwrap();
        stream.write(buff_to_read.as_bytes()).expect("Failed to write response!");

    }
    
}

fn serial_port_access(buffer:Buff){
    let mut port = match serialport::new("/dev/ttyUSB0", 115200)
    .timeout(std::time::Duration::from_millis(10))
    .open()
    {
        Ok(port) => port,
        Err(err) => panic!("Failed to open serial port: {}", err),
    };
    let mut serial_buf: Vec<u8> = vec![0; 1024];
    loop {
        thread::sleep(std::time::Duration::from_secs(1));
        match port.read(serial_buf.as_mut_slice()) {
            Ok(bytes_read) => {
                let mut writer = buffer.write().unwrap();
                *writer = String::from_utf8_lossy(&serial_buf[..bytes_read]).to_string();
                println!("{}",*writer)
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                    Err(e) => eprintln!("{:?}", e),
                };
    }
}

fn websocket_command(buffer : Buff , log_command:SharedData){
    let server = Server::bind("127.0.0.1:2794").unwrap();

	for request in server.filter_map(Result::ok) {
		let command = log_command.clone();
        // Spawn a new thread for each connection.
		thread::spawn(move|| {
			if !request.protocols().contains(&"rust-websocket".to_string()) {
				request.reject().unwrap();
				return;
			}

			let mut client = request.use_protocol("rust-websocket").accept().unwrap();

			let ip = client.peer_addr().unwrap();

			println!("Connection from {}", ip);

			let message = OwnedMessage::Text("Hello".to_string());
			client.send_message(&message).unwrap();

			let (mut receiver, mut sender) = client.split().unwrap();

			for message in receiver.incoming_messages() {
				let message = message.unwrap();

				match message {
					OwnedMessage::Close(_) => {
						let message = OwnedMessage::Close(None);
						sender.send_message(&message).unwrap();
						println!("Client {} disconnected", ip);
						return;
					}
					OwnedMessage::Ping(ping) => {
						let message = OwnedMessage::Pong(ping);
						sender.send_message(&message).unwrap();
					}
					_ => {
                        sender.send_message(&message).unwrap();
                        let msg = String::from_utf8(message.take_payload()).unwrap();
                        if msg == String::from("log-on"){
                            let commands = command.data.read().unwrap();
                            if commands.running == true{
                                sender.send_message(&OwnedMessage::Text(String::from("logging is already on!"))).unwrap();
                            }
                            else{
                                
                                println!("{}","log is on from now.");
                            }

                        }
                        if msg == String::from("log-off"){
                            println!("{}","log is off from now.");
                        }
                        else if msg == String::from("log-stats") {
                            println!("{}","log stats is ");
                        }
                    }
				}
			}
		});
	}
  
    //   impl Handler for Server {
    //       fn on_message(&mut self, msg: Message) -> Result<()> {
    //           println!("Server got message '{}'. ", msg);
    //           if msg.as_text().unwrap() == String::from("log-on"){
    //               match commands {
                    
    //             }
    //             println!("{}","log is on from now.");
    //           }
    //           if msg.as_text().unwrap() == String::from("log-off"){
    //             println!("{}","log is off from now.");
    //           }
    //           else if msg.as_text().unwrap() == String::from("log-stats") {
    //             println!("{}","log stats is ");
    //           }
    //           self.out.send(msg)
    //       }
  
    //       fn on_close(&mut self, code: CloseCode, reason: &str) {
    //           println!("WebSocket closing for ({:?}) {}", code, reason);
    //       }
    //   }
  
  
}


fn main(){
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Failed to bind to address");
    println!("Server listening on 127.0.0.1:8080");
    let buffer =  Buff::default();
    let log_command = SharedData::default();
    let buff_to_write = buffer.clone();
    thread::spawn(move||serial_port_access(buff_to_write));
    let buff_to_read_ws = buffer.clone();
    let command = log_command.clone();
    thread::spawn(move|| websocket_command(buff_to_read_ws,command));
    for stream in listener.incoming(){
        match stream{
            Ok(stream) => {
                let buff_to_read = buffer.clone();
                thread::spawn(move || handle_client(stream,buff_to_read));
            }
            Err(e) => {
                eprintln!("Failed to establish connection: {}", e);
            // stderr - standard error stream
            }
        }
    }
}
