#![allow(non_snake_case)]
use std::fs::File;
use serde::ser::{SerializeStruct, Serializer};
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
use std::mem;
use std::slice;
use std::collections::HashMap;


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
#[derive(Debug, Copy, Clone)]
struct SSVSNRData309 {
    m_wSYS_PRNID: u16,
    m_wStatus: u16,
    m_chElev: i8,
    m_byAzimuth: u8,
    m_wLower2BitsSNR7_6_5_4_3_2_1_0: u16,
    m_abySNR8Bits: [u32; 8],
}
impl Serialize for SSVSNRData309 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let sys = (self.m_wSYS_PRNID >> 7) & 0xF;
        let spare = (self.m_wSYS_PRNID >> 11) & 0x1F;
        let sat = match sys{
            0 => "GPS",
            1 => "GLONASS",
            2 => "GALILEO",
            3 => "BEIDOU",
            4 => "QZSS",
            5 => "IRNSS",
            7 => "SBAS",
            _ => "unknown" 
        };

    let mut signal_map: HashMap<&str, HashMap<&str, f32>> = HashMap::new();

    // Insert GPS signal IDs
    let mut gps_signals = HashMap::new();
    gps_signals.insert("L1CA", 0.0);
    gps_signals.insert("L2P", 0.0);
    gps_signals.insert("L2C", 0.0);
    gps_signals.insert("L5", 0.0);
    gps_signals.insert("L1C", 0.0);
    signal_map.insert("GPS", gps_signals);

    // Insert GLO signal IDs
    let mut glo_signals = HashMap::new();
    glo_signals.insert("G1C_G1P", 0.0);
    glo_signals.insert("G2C_G2P", 0.0);
    glo_signals.insert("G10C", 0.0);
    glo_signals.insert("G20C", 0.0);
    glo_signals.insert("G30C", 0.0);
    signal_map.insert("GLONASS", glo_signals);

    // Insert GAL signal IDs
    let mut gal_signals = HashMap::new();
    gal_signals.insert("E1BC", 0.0);
    gal_signals.insert("E5A", 0.0);
    gal_signals.insert("E5B", 0.0);
    gal_signals.insert("E6", 0.0);
    gal_signals.insert("ALTBOC", 0.0);
    signal_map.insert("GALILEO", gal_signals);

    // Insert BDS signal IDs
    let mut bds_signals = HashMap::new();
    bds_signals.insert("B1I", 0.0);
    bds_signals.insert("B2I", 0.0);
    bds_signals.insert("B3I", 0.0);
    bds_signals.insert("B1BOC", 0.0);
    bds_signals.insert("B2A", 0.0);
    bds_signals.insert("B2B", 0.0);
    bds_signals.insert("B3C", 0.0);
    bds_signals.insert("ACEBOC", 0.0);
    signal_map.insert("BEIDOU", bds_signals);

    // Insert QZS signal IDs
    let mut qzs_signals = HashMap::new();
    qzs_signals.insert("L1CA", 0.0);
    qzs_signals.insert("L2C", 0.0);
    qzs_signals.insert("L5", 0.0);
    qzs_signals.insert("L1C", 0.0);
    qzs_signals.insert("LEX", 0.0);
    signal_map.insert("QZSS", qzs_signals);

    // Insert IRN signal IDs
    let mut irn_signals = HashMap::new();
    irn_signals.insert("L5", 0.0);
    signal_map.insert("IRNSS", irn_signals);
        // Calculate SNR values
        let mut snr_values = [0f32; 8];
        for i in 0..8 {
            let lower2bits = (self.m_wLower2BitsSNR7_6_5_4_3_2_1_0 >> (2 * i)) & 0x3;
            if self.m_abySNR8Bits[i] == 0 {
                continue;
            }
            snr_values[i] = (((self.m_abySNR8Bits[i] as u64) << 2) + lower2bits as u64) as f32;
            snr_values[i] = 10.0*(0.8192 * snr_values[i] as f32).log10() + 30.0;
            // println!("snr value : {}", snr_values[i]);
            
        }

        for (system,signals) in &mut signal_map{
            if sat == *system{
                let mut counter = 0;
                for (_, value) in signals.iter_mut() {
                    // Modify the value
                    *value = snr_values[counter];
                    counter = counter + 1; 
                }
                break;
            }
        }
        let mut lock = [0u16; 8];
        // Bits 0-7: Code and Carrier Lock on Signal 0-7
        for i in 0..8 {
            lock[i] = (self.m_wStatus >> i) & 1;
        }

        // Bits 8-10: Bit Lock and Frame lock (decoding data) on Signal 0-2
        for i in 0..3 {
            let bit_lock = (self.m_wStatus >> (8 + i)) & 1;
        }

        // Bits 11-12: Spare
        let spare11 = (self.m_wStatus >> 11) & 1;
        let spare12 = (self.m_wStatus >> 12) & 1;
        // Bit 13: Ephemeris Available
        let ephemeris_available = match (self.m_wStatus >> 13) & 1{
            1=>"Yes",
            0=>"No",
            _ => "unknown",
        };
        

        let health_ok = match (self.m_wStatus >> 14) & 1{
            1=>"Yes",
            0=>"No",
            _ => "unknown",
        };

        let satellite_used = match (self.m_wStatus >> 15) & 1{
            1=>"Yes",
            0=>"No",
            _ => "unknown",
        };
        let el : u16 = 63;
        let temp = self.m_wSYS_PRNID & el;
        // println!("befor and {}",self.m_wSYS_PRNID);
        // println!("after and {}",temp);

        // Serialize the struct
        let mut state = serializer.serialize_struct("SSVSNRData309", 10)?;
        state.serialize_field("m_wSYS_PRNID", &(&temp))?;
        state.serialize_field("PRNID", &sat)?;
        state.serialize_field("SYS", &sys)?;
        state.serialize_field("Spare", &spare)?;
        state.serialize_field("m_wStatus", &self.m_wStatus)?;
        state.serialize_field("m_chElev", &self.m_chElev)?;
        state.serialize_field("m_byAzimuth", &(self.m_byAzimuth as u32 * 2.to_owned()) )?;
        state.serialize_field("m_wLower2BitsSNR7_6_5_4_3_2_1_0", &self.m_wLower2BitsSNR7_6_5_4_3_2_1_0)?;
        state.serialize_field("m_abySNR8Bits", &self.m_abySNR8Bits)?;
        state.serialize_field("Health Ok:", &health_ok)?;
        state.serialize_field("Ephimeris Available:", &ephemeris_available)?;
        state.serialize_field("Satellite used in Navigation Solution:", &satellite_used)?;
        println!("key is {}", sat);
        if sat == "SBAS"{
            state.serialize_field("SNR Values", &snr_values)?;    
        }
        else {

            state.serialize_field("SNR Values", &signal_map[sat])?;
        }
        state.end()
    }
}


#[repr(C)]
#[derive(Debug, Copy, Clone)]
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

impl Serialize for SBinaryMsg309 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let calc_checksum = self.calculate_checksum();
        let mut state = serializer.serialize_struct("SBinaryMsg309", 8)?;
        state.serialize_field("m_sHead", &self.m_sHead)?;
        state.serialize_field("m_dGPSTimeOfWeek", &self.m_dGPSTimeOfWeek)?;
        state.serialize_field("m_wGPSWeek", &self.m_wGPSWeek)?;
        state.serialize_field("m_cUTCTimeDiff", &self.m_cUTCTimeDiff)?;
        state.serialize_field("m_byPage", &self.m_byPage)?;
        state.serialize_field("m_asSVData309", &self.m_asSVData309)?;
        state.serialize_field("m_wCheckSum", &self.m_wCheckSum)?;
        state.serialize_field("calculated checksum", &calc_checksum)?;
        state.serialize_field("m_wCRLF", &self.m_wCRLF)?;
        state.end()
    }
}
impl SBinaryMsg309 {
    fn calculate_checksum(&self) -> u16 {
        let size_of_struct = mem::size_of::<SBinaryMsg309>();
        let offset_of_checksum = mem::offset_of!(SBinaryMsg309, m_wCheckSum);
        let struct_ptr = self as *const SBinaryMsg309 as *const u8;
        let struct_bytes = unsafe { slice::from_raw_parts(struct_ptr, size_of_struct) };
        let mut checksum: u16 = 0;
        let mut iteration_num = 0;
        for (i, &byte) in struct_bytes.iter().enumerate() {
            if i < offset_of_checksum || i >= offset_of_checksum + mem::size_of::<u16>() {
                checksum = checksum.wrapping_add(byte as u16);
                iteration_num = iteration_num + 1;
                
            }
        }
        // print!("checksum is  {}*******************************with iter {}",checksum,iteration_num);
        self.m_wCheckSum
    }
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
    let mut snr_values = [0u8; 8];
    cursor.read_exact(&mut snr_values)?;
    let mut m_abySNR8Bits = [0u32; 8];
    for i in 0..8 {
        let lower2bits = (m_wLower2BitsSNR7_6_5_4_3_2_1_0 >> (2 * i)) & 0x3;
        m_abySNR8Bits[i] = ((snr_values[i] as u32) << 2) + lower2bits as u32;
    }
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
        m_abySNR8Bits: [0u32; 8],
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
    loop {
        thread::sleep(Duration::from_secs(2));
        let binding = buffer.read().unwrap();
        let mut cursor = Cursor::new(binding.to_owned());
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
                let temp = &serial_buf[..bytes_read];
                println!("BIN {:?}",temp.to_vec());
                let bin_sequence = b"$BIN";
                let newline_sequence = b"\r\n";
                
                // Find the index of the $BIN sequence
                let bin_index = temp.windows(bin_sequence.len())
                .position(|window| window == bin_sequence);
            
            // Find the index of the \r\n sequence
            let newline_index = temp.windows(newline_sequence.len())
            .position(|window| window == newline_sequence);
        
            *writer = temp[bin_index.unwrap()..newline_index.unwrap()+2].to_vec();
            // let temp2 = String::from_utf8_lossy(&writer).to_string();
            println!("{:?}",writer);
                  // Check that the vector has at least 4 elements
                if writer.len() < 4 {
                    panic!("The vector must contain at least 4 elements.");
                }

                // Slice the vector to exclude the last 4 elements
                // let only_head = &writer[..&writer.len() - 4];

                // let data = &writer[20..&writer.len()-4];

                // Iterate over the slice and sum the bytes
                let mut sum: u16 = 0;
                for i in 0..=writer.len()-5 {
                    sum = sum + writer[i] as u16;
                }
                
                
                // let head_sum: u16 = only_head.iter().map(|&byte| {
                //     println!("{}", byte);
                //     byte as u16
                // }).sum();
                // let data_sum : u16 = data.iter().map(|&byte| byte as u16).sum();

                println!("checksum calculated is : {}",sum);
            }
            
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                    Err(e) => eprintln!("{:?}", e),
                };
            }
        }
        
