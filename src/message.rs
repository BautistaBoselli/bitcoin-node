use crate::error::CustomError;
pub trait Message {
    fn serialize(&self) -> Vec<u8>;
    fn get_command(&self) -> String;
    fn parse(buffer: Vec<u8>) -> Result<Box<Self>, CustomError>;
}
