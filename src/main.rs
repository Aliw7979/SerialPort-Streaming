#![allow(non_snake_case)]
use std::fs::File;

use std::io::Read;
use std::sync::RwLock;
use std::sync::Arc;
use std::time::Duration;
use serialport::{self};
use std::{io, thread};
use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt};
use std::fmt;
type Buff = Arc<RwLock<Vec<u8>>>;
use serde::Serialize;
use std::io::Write;

#[repr(C)]
#[derive(Copy, Clone, Serialize)]
struct SBinaryMsgHeader {
    m_strSOH: [u8; 4],
    m_byBlockID: u16,
    m_wDataLength: u16,
}

impl fmt::Debug for SBinaryMsgHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SBinaryMsgHeader {{ m_strSOH: {:?}, m_byBlockID: {:?}, m_wDataLength: {:?} }}",
            std::str::from_utf8(&self.m_strSOH).unwrap_or("Invalid UTF-8"),
            self.m_byBlockID,
            self.m_wDataLength
        )
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Serialize)]
struct SBinaryMsgHeaderDW {
    ulDwordPreamble: u32,
    ulDwordInfo: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
union SUnionMsgHeader {
    sBytes: SBinaryMsgHeader,
    sDWord: SBinaryMsgHeaderDW,
}

impl fmt::Debug for SUnionMsgHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            if std::str::from_utf8(&self.sBytes.m_strSOH).unwrap_or("") == "$BIN" {
                write!(f, "SUnionMsgHeader {{ sBytes: {:?} }}", self.sBytes)
            } else {
                write!(f, "SUnionMsgHeader {{ sDWord: {:?} }}", self.sDWord)
            }
        }
    }
}

// Manual implementation of Serialize for the union
impl Serialize for SUnionMsgHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        unsafe {
            if std::str::from_utf8(&self.sBytes.m_strSOH).unwrap_or("") == "$BIN" {
                self.sBytes.serialize(serializer)
            } else {
                self.sDWord.serialize(serializer)
            }
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Serialize)]
struct SSVSNRData309 {
    m_wSYS_PRNID: u16,
    m_wStatus: u16,
    m_chElev: i8,
    m_byAzimuth: u8,
    m_wLower2BitsSNR7_6_5_4_3_2_1_0: u16,
    m_abySNR8Bits: [u8; 8],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Serialize)]
struct SBinaryMsg309 {
    m_sHead: SUnionMsgHeader,
    m_dGPSTimeOfWeek: f64,
    m_wGPSWeek: u16,
    m_cUTCTimeDiff: i8,
    m_byPage: u8,
    m_asSVData309: [SSVSNRData309; 30],
    m_wCheckSum: u16,
    m_wCRLF: u16,
}


fn parse_sbinary_msg_header(cursor: &mut std::io::Cursor<Vec<u8>>) -> io::Result<SBinaryMsgHeader> {
    let mut m_strSOH = [0u8; 4];
    cursor.read_exact(&mut m_strSOH)?;
    let m_byBlockID = cursor.read_u16::<LittleEndian>()?;
    let m_wDataLength = cursor.read_u16::<LittleEndian>()?;
    Ok(SBinaryMsgHeader {
        m_strSOH,
        m_byBlockID,
        m_wDataLength,
    })
}

fn parse_sbinary_msg_headerdw(cursor: &mut std::io::Cursor<Vec<u8>>) -> io::Result<SBinaryMsgHeaderDW> {
    let ulDwordPreamble = cursor.read_u32::<LittleEndian>()?;
    let ulDwordInfo = cursor.read_u32::<LittleEndian>()?;
    Ok(SBinaryMsgHeaderDW {
        ulDwordPreamble,
        ulDwordInfo,
    })
}

fn parse_ssvsnr_data309(cursor: &mut std::io::Cursor<Vec<u8>>) -> io::Result<SSVSNRData309> {
    let m_wSYS_PRNID = cursor.read_u16::<LittleEndian>()?;
    let m_wStatus = cursor.read_u16::<LittleEndian>()?;
    let m_chElev = cursor.read_i8()?;
    let m_byAzimuth = cursor.read_u8()?;
    let m_wLower2BitsSNR7_6_5_4_3_2_1_0 = cursor.read_u16::<LittleEndian>()?;
    let mut m_abySNR8Bits = [0u8; 8];
    cursor.read_exact(&mut m_abySNR8Bits)?;
    Ok(SSVSNRData309 {
        m_wSYS_PRNID,
        m_wStatus,
        m_chElev,
        m_byAzimuth,
        m_wLower2BitsSNR7_6_5_4_3_2_1_0,
        m_abySNR8Bits,
    })
}

fn parse_sbinary_msg309(cursor: &mut std::io::Cursor<Vec<u8>>)-> io::Result<SBinaryMsg309> {
    let m_sHead: SUnionMsgHeader =  {
        if cursor.get_ref().len() - cursor.position() as usize >= 8 {
            let pos = cursor.position() as usize;
            let slice = &cursor.get_ref()[pos..pos + 8];
            if slice.starts_with(b"$BIN") {
                SUnionMsgHeader {
                    sBytes: parse_sbinary_msg_header(cursor)?,
                }
            } else {
                SUnionMsgHeader {
                    sDWord: parse_sbinary_msg_headerdw(cursor)?,
                }
            }
        } 
        else {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Not enough data for union"));
        }
     
    };

    let m_dGPSTimeOfWeek = cursor.read_f64::<LittleEndian>()?;
    let m_wGPSWeek = cursor.read_u16::<LittleEndian>()?;
    let m_cUTCTimeDiff = cursor.read_i8()?;
    let m_byPage = cursor.read_u8()?;

    let mut m_asSVData309 = [SSVSNRData309 {
        m_wSYS_PRNID: 0,
        m_wStatus: 0,
        m_chElev: 0,
        m_byAzimuth: 0,
        m_wLower2BitsSNR7_6_5_4_3_2_1_0: 0,
        m_abySNR8Bits: [0u8; 8],
    }; 30];
    
    for data in &mut m_asSVData309 {
        *data = parse_ssvsnr_data309(cursor)?;
    }

    let m_wCheckSum = cursor.read_u16::<LittleEndian>()?;
    let m_wCRLF = cursor.read_u16::<LittleEndian>()?;
    let mut bufbufac : Vec<u8> = vec![0; 1024]; 
    let _ = cursor.read_to_end(&mut bufbufac);
    Ok(SBinaryMsg309 {
        m_sHead,
        m_dGPSTimeOfWeek,
        m_wGPSWeek,
        m_cUTCTimeDiff,
        m_byPage,
        m_asSVData309,
        m_wCheckSum,
        m_wCRLF,
    })
}

fn main() -> io::Result<()> {
    let buffer:Buff = Buff::default();
    let buff_to_write = buffer.clone();
    thread::spawn(move||serial_port_access(buff_to_write));
    // Sample binary data (must be replaced with actual data)
    loop {
        // println!("{:?}",buffer.read().unwrap());
        thread::sleep(Duration::from_secs(2));
        let binding = buffer.read().unwrap();
        // println!("this is the value of binding {:?}", binding);
        let mut cursor = Cursor::new(binding.to_owned());
        // let binding = buff_to_deser.read().unwrap();
        // println!("this is the value of binding {:?}", binding);
        // let mut cursor = Cursor::new(binding.as_slice());
        let sbinary_msg = parse_sbinary_msg309(&mut cursor)?;
        println!("result : {:?}",sbinary_msg);
        let json = serde_json::to_string_pretty(&sbinary_msg).unwrap();
        let mut file = File::create("output.json").unwrap();
        file.write_all(json.as_bytes()).unwrap();
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
        match port.read(&mut serial_buf) {
            Ok(bytes_read) => {
                
                let mut writer = buffer.write().unwrap();
                // let temp = String::from_utf8_lossy(&serial_buf[..bytes_read]).to_string();
                let temp = &serial_buf[..bytes_read];
                let bin_sequence = b"$BIN";
                let newline_sequence = b"\r\n";
            
                // Find the index of the $BIN sequence
                let bin_index = temp.windows(bin_sequence.len())
                                    .position(|window| window == bin_sequence);
            
                // Find the index of the \r\n sequence
                let newline_index = temp.windows(newline_sequence.len())
                                        .position(|window| window == newline_sequence);
            
                *writer = temp[bin_index.unwrap()..newline_index.unwrap()+2].to_vec();
                println!("{:?}",writer);
            }
            
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                    Err(e) => eprintln!("{:?}", e),
                };
            }
        }
        
