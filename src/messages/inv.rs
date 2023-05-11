use crate::{error::CustomError, message::Message, parser::BufferParser, parser::VarIntSerialize};

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
        buffer.extend(self.inventories.len().to_varint_bytes());
        for inventory in &self.inventories {
            buffer.extend(inventory.serialize());
        }
        buffer
    }

    fn get_command(&self) -> String {
        String::from("inv")
    }

    fn parse(buffer: Vec<u8>) -> Result<Self, crate::error::CustomError> {
        let mut parser = BufferParser::new(buffer);

        let inventory_count = parser.extract_varint()?;

        if parser.len() % 36 != 0 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        println!("inventory count: {}", inventory_count);

        let mut inventories = vec![];
        while !parser.is_empty() {
            inventories.push(Inventory::parse(parser.extract_buffer(36)?.to_vec())?);
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
            InventoryType::GetBlock => 2_u32,
        };
        buffer.extend(inventory_type.to_le_bytes());
        buffer.extend(&self.hash);
        buffer
    }

    pub fn parse(buffer: Vec<u8>) -> Result<Self, CustomError> {
        let mut parser = BufferParser::new(buffer);
        if parser.len() != 36 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }
        let inventory_type = match parser.extract_u32()? {
            2_u32 => InventoryType::GetBlock,
            _ => return Err(CustomError::SerializedBufferIsInvalid),
        };
        Ok(Self {
            inventory_type,
            hash: parser.extract_buffer(32)?.to_vec(),
        })
    }
}
