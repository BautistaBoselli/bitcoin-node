use crate::{error::CustomError, message::Message, messages::headers::parse_var_int};

use super::headers::serialize_var_int;

pub struct Inv {
    pub inventories: Vec<Inventory>,
}

impl Inv {
    pub fn new(inventories: Vec<Inventory>) -> Self {
        Self { inventories }
    }
}

impl Message for Inv {
    fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(serialize_var_int(self.inventories.len() as u64));
        for inventory in &self.inventories {
            buffer.extend(inventory.serialize());
        }
        buffer
    }

    fn get_command(&self) -> String {
        String::from("inv")
    }

    fn parse(buffer: Vec<u8>) -> Result<Self, crate::error::CustomError>
    where
        Self: Sized,
    {
        if buffer.len() == 0 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        let (inventory_count, mut i) = parse_var_int(&buffer);

        if (buffer.len() - i) % 36 != 0 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        println!("inventory count: {}", inventory_count);

        let mut inventories = vec![];
        while i < buffer.len() {
            inventories.push(Inventory::parse(buffer[i..(i + 36)].to_vec())?);
            i += 36;
        }
        Ok(Self { inventories })
    }
}

#[derive(Debug, Clone)]
pub enum InventoryType {
    GetBlock,
}

#[derive(Debug, Clone)]
pub struct Inventory {
    pub inventory_type: InventoryType,
    pub hash: Vec<u8>,
}

impl Inventory {
    pub fn new(inventory_type: InventoryType, hash: Vec<u8>) -> Self {
        Self {
            inventory_type,
            hash,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        let inventory_type = match self.inventory_type {
            InventoryType::GetBlock => 2 as u32,
        };
        buffer.extend(inventory_type.to_le_bytes());
        buffer.extend(&self.hash);
        buffer
    }

    pub fn parse(buffer: Vec<u8>) -> Result<Self, CustomError> {
        if buffer.len() != 36 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }
        let inventory_type = match u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]])
        {
            2 => InventoryType::GetBlock,
            _ => return Err(CustomError::SerializedBufferIsInvalid),
        };
        Ok(Self {
            inventory_type,
            hash: buffer[4..].to_vec(),
        })
    }
}
