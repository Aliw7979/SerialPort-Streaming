// Importing necessary modules from the Rust libraries
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::RwLock;
use std::sync::Arc;
use serialport;
use std::{io, thread};

type Buff = Arc<RwLock<String>>;


fn handle_client(mut stream: TcpStream,buffer : Buff){
    // this is a buffer to read data from the client
    // let mut buffer = [0; 1024];
    // this line reads data from the stream and stores it in the buffer.
    // stream.read(&mut buffer).expect("Failed to read from client!");
    // this line converts the data in the buffer into a UTF-8 enccoded string.
    // let request = String::from_utf8_lossy(&buffer[..]);
    // println!("Received request: {}", request);
    let buff_to_read = buffer.read().unwrap();
    
    stream.write(buff_to_read.as_bytes()).expect("Failed to write response!");
}

fn serial_port_access(buffer:Buff){
    let mut port = match serialport::new("/dev/ttyUSB0", 115200)
    .timeout(std::time::Duration::from_secs(5))
    .open()
    {
        Ok(port) => port,
        Err(err) => panic!("Failed to open serial port: {}", err),
    };
    let mut serial_buf: Vec<u8> = vec![0; 512];
    loop {
        match port.read(serial_buf.as_mut_slice()) {
            Ok(bytes_read) => {
                let mut writer = buffer.write().unwrap();
                *writer = String::from_utf8_lossy(&serial_buf[..bytes_read]).to_string();
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                    Err(e) => eprintln!("{:?}", e),
                };
    }
}
// Entry point
fn main(){
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Failed to bind to address");
    println!("Server listening on 127.0.0.1:8080");
    let buffer =  Buff::default();
    let buff_to_write = buffer.clone();
    thread::spawn(move||serial_port_access(buff_to_write));
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