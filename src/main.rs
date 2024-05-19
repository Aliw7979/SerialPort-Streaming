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
use chrono::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Result as res;
use websocket::sync::Server;
use websocket::OwnedMessage;

#[derive(Debug, Default,Clone)]
struct FormData {
    running : bool,
    session_name: String,
    file_len : String,
}

#[derive(Debug, Default, Clone)]
struct FormDataShare{
    form : Arc<RwLock<FormData>>,
}

impl FormDataShare {
    // Method to reset the struct to its default state
    fn reset(&mut self) {
        *self = Default::default();
    }
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

fn log_creator(file_name : String,path : PathBuf){
        //create directory
        if path.exists() {
            println!("Path exists log error at address: {}", path.display());
            }
        else {
                println!("Path does not exist: {}", path.display());
            
                match fs::create_dir(path.clone()) {
                    Ok(_) =>{
                        println!("Directory created successfully");
                    } 
                    Err(err) => println!("Error creating directory: {}", err),
                }
                
        }
        // create file
        let mut path_to_create = path.clone();
        path_to_create.push("/".to_owned() + file_name.as_str()); // Specify the file path here    
        match fs::File::create(path_to_create) {
        Ok(file) =>{
            file
        }
        Err(err) => {
            println!("Error creating file: {}", err);
            return;
        }
        };
}

fn log_writer(form : FormDataShare , buffer : Buff){
    let mut is_init = false;
    let mut file_name = "";
    let mut new_file = true;
    let mut decoded_len_number = 0;
    let mut decoded_len_scale = "";
    while form.form.read().unwrap().running == true{
        let splited = buffer.read().unwrap();
        let date = splited.split(",").nth(2).unwrap();
        let hours = &date[0..2];
        let minutes = &date[2..4];
        let seconds = &date[4..6];
        
        // Convert to integers
        let ihours: u32 = hours.parse().unwrap();
        let iminutes: u32 = minutes.parse().unwrap();
        let iseconds: u32 = seconds.parse().unwrap();

        let datetime = Utc::now();
        let mut path = PathBuf::new();
        path.push("log");
        path.push(datetime.year().to_string());
        path.push(datetime.month().to_string());
        path.push(datetime.day().to_string());
        path.push(form.form.read().unwrap().session_name.clone());
        
        if is_init == false{
            log_creator(date.to_string(), path)
        }
        if form.form.read().unwrap().file_len.contains("h"){
            
        }
        else if form.form.read().unwrap().file_len.contains("m"){
            
        }
        else if form.form.read().unwrap().file_len.contains("s"){
            
        }


    }
}

fn get_directory_size(path: &Path) -> io::Result<u64> {
    let mut total_size = 0;

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                total_size += get_directory_size(&path)?;
            } else {
                total_size += entry.metadata()?.len();
            }
        }
    } else {
        total_size += path.metadata()?.len();
    }

    Ok(total_size)
}

fn websocket_command(buffer : Buff ){
    let server = Server::bind("127.0.0.1:2794").unwrap();
	for request in server.filter_map(Result::ok) {
        // Spawn a new thread for each connection.
		let command = FormDataShare::default();
        let mut form = command.clone();
        let buffer_to_read = buffer.clone();
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
            let form_to_read = form.clone();
            thread::spawn(move||log_writer(form_to_read, buffer_to_read));

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
                        if msg.contains("log-on"){
                            if form.form.read().unwrap().running == true{
                                sender.send_message(&OwnedMessage::Text(String::from("logging is already on!"))).unwrap();
                            }
                            else{
                                form.reset();
                                let fill_form = form.form.read().unwrap();
                                let splited = msg.split(" ").skip(1);
                                for sp in splited{
                                    if fill_form.session_name == ""{
                                    let mut write_on_form = form.form.write().unwrap();
                                    write_on_form.session_name = sp.to_string();
                                    }
                                    if fill_form.file_len == ""{
                                    let mut write_on_form = form.form.write().unwrap();
                                    write_on_form.file_len = sp.to_string();
                                    }
                                }
                                let mut write_on_form = form.form.write().unwrap();
                                write_on_form.running = true;
                                println!("{}","log is on from now.");
                                
                            }

                        }
                        if msg == String::from("log-off"){
                            let mut write_on_form = form.form.write().unwrap();
                            write_on_form.running = false;
                            println!("{}","log is off from now.");
                        }
                        else if msg == String::from("log-stats") {
                            let datetime = Utc::now();
                            let mut path = PathBuf::new();
                            path.push("log");
                            path.push(datetime.year().to_string());
                            path.push(datetime.month().to_string());
                            path.push(datetime.day().to_string());
                            path.push(form.form.read().unwrap().session_name.clone());
                            if path.exists() {
                                println!("Path exists: {}", path.display());
                                match get_directory_size(&path) {
                                    Ok(size) => {
                                        let size_in_mb = size as f64 / (1024.0 * 1024.0);
                                        println!("Directory size: {:.2} MB", size_in_mb);
                                    }
                                    Err(e) => println!("Failed to calculate directory size: {}", e),
                                }
                                } 
                            else {
                                    println!("Path does not exist: {}", path.display());
                                
                            }
                            println!("{}","log stats is ");
                        }
                    }
				}
			}
		});
	}
  
  
}


fn main(){
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Failed to bind to address");
    println!("Server listening on 127.0.0.1:8080");
    let buffer =  Buff::default();
    let buff_to_write = buffer.clone();
    thread::spawn(move||serial_port_access(buff_to_write));
    let buff_to_read_ws = buffer.clone();
    thread::spawn(move|| websocket_command(buff_to_read_ws));
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
