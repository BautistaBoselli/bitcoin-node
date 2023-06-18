use crate::{
    error::CustomError,
    messages::transaction::{OutPoint, TransactionOutput},
    parser::BufferParser,
};
use std::collections::HashMap;

pub fn serialize_utxo(out_point: &OutPoint, tx: &TransactionOutput) -> Vec<u8> {
    let mut buffer: Vec<u8> = vec![];
    buffer.extend(out_point.serialize());
    buffer.extend(tx.serialize());
    buffer
}

pub fn parse_utxo(buffer: Vec<u8>) -> Result<HashMap<OutPoint, TransactionOutput>, CustomError> {
    let mut parser = BufferParser::new(buffer);

    let mut utxo_set: HashMap<OutPoint, TransactionOutput> = HashMap::new();
    while !parser.is_empty() {
        let out_point = OutPoint::parse(parser.extract_buffer(36)?.to_vec())?;
        let tx = TransactionOutput::parse(&mut parser)?;
        utxo_set.insert(out_point, tx);
    }
    Ok(utxo_set)
}
