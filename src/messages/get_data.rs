use crate::message::Message;

use super::inv::{Inv, Inventory};

pub struct GetData {
    inv: Inv,
}

impl GetData {
    pub fn new(inventories: Vec<Inventory>) -> Self {
        let inv = Inv::new(inventories);
        Self { inv }
    }
}

impl Message for GetData {
    fn serialize(&self) -> Vec<u8> {
        self.inv.serialize()
    }

    fn get_command(&self) -> String {
        String::from("getdata")
    }

    fn parse(buffer: Vec<u8>) -> Result<Self, crate::error::CustomError> {
        Ok(Self {
            inv: Inv::parse(buffer)?,
        })
    }
}
