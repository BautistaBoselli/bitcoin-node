use crate::{error::CustomError, message::Message, parser::BufferParser, parser::VarIntSerialize};

#[derive(Debug, PartialEq)]
///Esta es la estructura de un mensaje inv, la cual contiene un vector de inventories
pub struct Inv {
    pub inventories: Vec<Inventory>,
}

impl Inv {
    pub fn new(inventories: Vec<Inventory>) -> Self {
        Self { inventories }
    }
}

///Implementa el trait Message para el mensaje inv.
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

        let count = parser.extract_varint()? as usize;

        if parser.len() % 36 != 0 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        let mut inventories = vec![];
        for _i in 0..count {
            inventories.push(Inventory::parse(parser.extract_buffer(36)?.to_vec())?);
        }
        Ok(Self { inventories })
    }
}

#[derive(Debug, Clone, PartialEq)]
///Este enum contiene los tipos de inventarios que se pueden enviar:
/// - Tx = 1
/// - Block = 2
/// - FilteredBlock = 3
/// - CompactBlock = 4
/// - WitnessTx = 5
/// - WitnessBlock = 6
/// - FilteredWitnessBlock = 7
pub enum InventoryType {
    Tx,
    Block,
    FilteredBlock,
    CompactBlock,
    WitnessTx,
    WitnessBlock,
    FilteredWitnessBlock,
}

#[derive(Debug, Clone, PartialEq)]
///Esta es la estructura de un inventario, la cual contiene un tipo de inventario y un hash del inventario en si.
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
            InventoryType::Tx => 1_u32,
            InventoryType::Block => 2_u32,
            InventoryType::FilteredBlock => 3_u32,
            InventoryType::CompactBlock => 4_u32,
            InventoryType::WitnessTx => 0x40000001,
            InventoryType::WitnessBlock => 0x40000002,
            InventoryType::FilteredWitnessBlock => 0x40000003,
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
            1_u32 => InventoryType::Tx,
            2_u32 => InventoryType::Block,
            3_u32 => InventoryType::FilteredBlock,
            4_u32 => InventoryType::CompactBlock,
            0x40000001 => InventoryType::WitnessTx,
            0x40000002 => InventoryType::WitnessBlock,
            0x40000003 => InventoryType::FilteredWitnessBlock,
            _ => {
                println!("inventory type: {}", parser.extract_u32()?);
                return Err(CustomError::SerializedBufferIsInvalid);
            }
        };
        Ok(Self {
            inventory_type,
            hash: parser.extract_buffer(32)?.to_vec(),
        })
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]
    fn inv_serialize_and_parse() {
        let inventory = Inventory {
            inventory_type: InventoryType::Block,
            hash: [
                220, 9, 210, 68, 121, 44, 33, 165, 243, 235, 248, 125, 43, 136, 39, 116, 186, 43,
                114, 204, 35, 144, 47, 194, 229, 44, 97, 83, 110, 112, 229, 230,
            ]
            .to_vec(),
        };
        let inv = Inv {
            inventories: vec![inventory],
        };

        let buffer = inv.serialize();
        let parsed_inv = Inv::parse(buffer).unwrap();

        assert_eq!(inv, parsed_inv);
    }

    #[test]
    fn inv_invalid_buffer() {
        let buffer = vec![
            1, 0, 0, 0, 5, 159, 141, 74, 195, 4, 19, 253, 127, 1, 148, 149, 222, 143, 237, 24, 27,
            124, 186, 34, 123, 241, 216, 166, 203, 239, 86, 108, 0, 0, 0, 0, 233, 233, 109, 115,
            249, 241, 6, 200, 176, 73, 10, 24, 28, 209, 102, 159, 255, 179, 239, 72, 185, 225, 10,
            14, 219,
        ];

        let inv = Inv::parse(buffer);

        assert!(inv.is_err());
    }
}
