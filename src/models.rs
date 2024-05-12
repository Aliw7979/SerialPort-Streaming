use prost::Message as Message;
use std::io::Cursor;
// Include the `items` module, which is generated from items.proto.
pub mod items {
    include!(concat!(env!("OUT_DIR"), "/items.rs"));
}
pub struct Gga {
    pub command:String,
    pub utc : String,
    pub latitude : String,
    pub ns_indicator: String,
    pub longitude : String,
    pub en_indicator : String,
    pub pfi : String,
    pub su : String,
    pub hdop : String,
    pub msl_altitude : String,
    pub units1 : String,
    pub geod : String,
    pub units2 : String,
    pub checksum : String,
}
pub struct InformationStruct{
    pub command                 : String,
    pub serial_number           : String,
    pub extended_data_1         : String,
    pub extended_data_2         : String,
    pub extended_data_3         : String,
    pub subscription_expire_date: String,
    pub configuration_code      : String,
    pub firmware_version_number : String,
}


// pub fn create_gpgga(gpgga: Gga) -> items::Gga {
//     let mut info = items::Gga::default();
//     info.command = gpgga.command ;
//     info.utc = gpgga.utc;
//     info.latitude = gpgga.latitude;
//     info.ns_indicator = gpgga.ns_indicator;
//     info.longitude = gpgga.longitude;
//     info.en_indicator = gpgga.en_indicator;
//     info.pfi = gpgga.pfi;
//     info.su = gpgga.su;
//     info.hdop = gpgga.hdop;
//     info.msl_altitude = gpgga.msl_altitude;
//     info.units = gpgga.units1;
//     info.geod = gpgga.geod;
//     info.units = gpgga.units2;
//     info.checksum = gpgga.checksum;
//     info
// }

// pub fn serialize_Gga(shirt: &items::Gga) -> Vec<u8> {
//     let mut buf = Vec::new();
//     buf.reserve(shirt.encoded_len());
//     shirt.encode(&mut buf).unwrap();
//     buf
// }

// pub fn deserialize(buf: &[u8]) -> 
//     Result<items::Information, prost::DecodeError> {
//     items::Information::decode(&mut Cursor::new(buf))
// }


pub fn create_info(infoStruct: InformationStruct) -> items::Information {
    let mut info = items::Information::default();
    info.command = infoStruct.command ;
    info.serial_number = infoStruct.serial_number;
    info.extended_data_1 = infoStruct.extended_data_1;
    info.extended_data_2 = infoStruct.extended_data_2;
    info.extended_data_3 = infoStruct.extended_data_3;
    info.subscription_expire_date = infoStruct.subscription_expire_date;
    info.configuration_code = infoStruct.configuration_code;
    info.firmware_version_number = infoStruct.firmware_version_number;
    info
}

pub fn serialize(shirt: &items::Information) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.reserve(shirt.encoded_len());
    shirt.encode(&mut buf).unwrap();
    buf
}

pub fn deserialize(buf: &[u8]) -> 
    Result<items::Information, prost::DecodeError> {
    items::Information::decode(&mut Cursor::new(buf))
}

#[cfg(test)]
mod tests {
    use crate::models::*;

    #[test]
    fn create_shirt() {
        let info = InformationStruct{
            command                 : String::from("test"),
            serial_number           : String::from("test"),
            extended_data_1         : String::from("test"),
            extended_data_2         : String::from("test"),
            extended_data_3         : String::from("test"),
            subscription_expire_date: String::from("test"),
            configuration_code      : String::from("test"),
            firmware_version_number : String::from("test"),
        };
        
        let infos = create_info(info);
        println!("info is {:?}", &infos);
        assert_eq!(infos.command, "test");
    }

    // #[test]
    // fn serde_shirt() {
    //     let shirt = create_large_shirt("white".to_string());
    //     let serded = deserialize_shirt(&serialize_shirt(&shirt))
    //       .expect("A shirt!");
    //     println!("Serded {:?}", serded);
    //     assert_eq!(serded, shirt);
    // }
}