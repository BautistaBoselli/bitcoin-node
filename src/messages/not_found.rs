use crate::{message::Message, structs::inventory::Inventory};

use super::inv::Inv;

/// Esta estructura representa al mensaje 'notfound' de Bitcoin, el cual se utiliza para notificar a un nodo que no se tiene el inventario solicitado
pub struct NotFound {
    inv: Inv,
}

impl NotFound {
    /// Esta funcion se encarga de crear un nuevo mensaje 'notfound' con un vector de inventarios que recibe por parametro
    pub fn new(inventories: Vec<Inventory>) -> Self {
        let inv = Inv::new(inventories);
        Self { inv }
    }

    /// Esta funcion se encarga de devolver el vector de inventarios del mensaje 'notfound'
    pub fn get_inventories(&self) -> &Vec<Inventory> {
        &self.inv.inventories
    }
}

/// Implementa el trait Message para el mensaje 'notfound'
/// Permite serializar, parsear y obtener el comando
impl Message for NotFound {
    fn serialize(&self) -> Vec<u8> {
        self.inv.serialize()
    }

    fn get_command(&self) -> String {
        String::from("notfound")
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
    fn serialize_not_found() {
        let inv = Inv::new(vec![Inventory::new(
            InventoryType::Block,
            vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xab, 0xcd],
        )]);
        let not_found = NotFound { inv };
        let serialized_not_found = not_found.serialize();
        assert_eq!(
            serialized_not_found,
            vec![1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xab, 0xcd,]
        );
    }

    #[test]
    fn parse_not_found() {
        let buffer = vec![
            1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xab, 0xcd, 4, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0,
            0, 0, 0xef, 0xaa, 3, 12, 7, 0, 7, 8,
        ];
        let not_found = NotFound::parse(buffer).unwrap();
        let inventories = not_found.get_inventories();
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
    fn get_command_not_found() {
        let not_found = NotFound::new(vec![]);
        assert_eq!(not_found.get_command(), "notfound");
    }
}
