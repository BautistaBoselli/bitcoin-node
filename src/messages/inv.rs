use crate::{
    error::CustomError, message::Message, parser::BufferParser, parser::VarIntSerialize,
    structs::inventory::Inventory,
};

#[derive(Debug, PartialEq)]
/// Esta es la estructura de un mensaje inv, la cual contiene un vector de inventories
pub struct Inv {
    pub inventories: Vec<Inventory>,
}

impl Inv {
    /// Esta funcion se encarga de crear un nuevo mensaje inv con un vector de inventories que recibe por parametro
    pub fn new(inventories: Vec<Inventory>) -> Self {
        Self { inventories }
    }
}

/// Implementa el trait Message para el mensaje inv.
/// Permite serializar, parsear y obtener el comando
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

#[cfg(test)]

mod tests {

    use crate::structs::inventory::InventoryType;

    use super::*;

    #[test]
    fn inv_serialize_and_parse() {
        let inventory = Inventory::new(
            InventoryType::Block,
            [
                220, 9, 210, 68, 121, 44, 33, 165, 243, 235, 28, 125, 43, 136, 29, 116, 190, 43,
                124, 200, 30, 144, 40, 190, 229, 44, 93, 83, 110, 112, 225, 220,
            ]
            .to_vec(),
        );
        let inv = Inv::new(vec![inventory]);
        let buffer = inv.serialize();
        let parsed_inv = Inv::parse(buffer).unwrap();
        assert_eq!(inv, parsed_inv);
    }

    #[test]
    fn inv_invalid_buffer() {
        let inventory = Inventory {
            inventory_type: InventoryType::Block,
            hash: [
                220, 9, 210, 68, 121, 44, 33, 165, 243, 235, 28, 125, 43, 136, 29, 116, 190, 43,
                124, 200, 30, 144, 40, 190, 229, 44, 93, 83, 110, 112, 225, 220, 100, 200, 129,
                233, 45, 56, 82, 56, 124, 200, 30, 144, 40, 190, 229, 44, 93, 83, 110, 112, 46,
            ]
            .to_vec(),
        };
        let inv = Inv::new(vec![inventory]);
        let buffer = inv.serialize();
        let parsed_inv = Inv::parse(buffer);
        assert!(parsed_inv.is_err());
    }

    #[test]
    fn get_command_inv() {
        let inv = Inv::new(vec![]);
        assert_eq!(inv.get_command(), "inv");
    }
}
