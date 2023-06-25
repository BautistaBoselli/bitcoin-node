use crate::{message::Message, structs::inventory::Inventory};

use super::inv::Inv;

pub struct GetData {
    inv: Inv,
}

impl GetData {
    pub fn new(inventories: Vec<Inventory>) -> Self {
        let inv = Inv::new(inventories);
        Self { inv }
    }

    pub fn get_inventories(&self) -> &Vec<Inventory> {
        &self.inv.inventories
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

#[cfg(test)]

mod tests {

    use crate::structs::inventory::{Inventory, InventoryType};

    use super::*;

    #[test]
    fn serialize_get_data() {
        let inv = Inv::new(vec![Inventory::new(
            InventoryType::Block,
            vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xab, 0xcd],
        )]);
        let get_data = GetData { inv };
        let serialized_get_data = get_data.serialize();
        assert_eq!(
            serialized_get_data,
            vec![1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xab, 0xcd,]
        );
    }

    #[test]
    fn parse_get_data() {
        let buffer = vec![
            1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xab, 0xcd, 4, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0,
            0, 0, 0xef, 0xaa, 3, 12, 7, 0, 7, 8,
        ];
        let get_data = GetData::parse(buffer).unwrap();
        let inventories = get_data.get_inventories();
        assert_eq!(inventories.len(), 1);
        assert_eq!(inventories[0].inventory_type, InventoryType::Block);
        assert_eq!(
            inventories[0].hash,
            vec![
                0, 0, 0, 0, 0, 0, 0, 0, 0xab, 0xcd, 4, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0xef,
                0xaa, 3, 12, 7, 0, 7, 8,
            ]
        );
    }

    #[test]
    fn get_command_get_data() {
        let get_data = GetData::new(vec![]);
        assert_eq!(get_data.get_command(), "getdata");
    }
}
