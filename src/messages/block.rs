use super::headers::BlockHeader;

use crate::{
    error::CustomError,
    message::Message,
    parser::{BufferParser, VarIntSerialize},
};

#[derive(Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Self {
            header,
            transactions,
        }
    }
}

impl Message for Block {
    fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(self.header.serialize());
        buffer.extend(self.transactions.len().to_varint_bytes());
        for transaction in &self.transactions {
            buffer.extend(transaction.serialize());
        }
        buffer
    }

    fn get_command(&self) -> String {
        String::from("block")
    }

    fn parse(buffer: Vec<u8>) -> Result<Self, crate::error::CustomError> {
        let mut parser = BufferParser::new(buffer);
        let header = BlockHeader::parse(parser.extract_buffer(80)?.to_vec(), false)?;
        let tx_count = parser.extract_varint()? as usize;
        let mut transactions = vec![];
        for _tx in 0..tx_count {
            let transaction = Transaction::parse(&mut parser)?;
            transactions.push(transaction);
        }
        // let (transaction_count, mut i) = parse_var_int(&buffer[80..]);
        // let mut transactions = vec![];
        // while i < buffer.len() {
        //     let transaction = Transaction::parse(buffer[i..].to_vec())?;
        //     transactions.push(transaction);
        //     i += 1;
        // }
        Ok(Self {
            header,
            transactions,
        })
    }
}

#[derive(Debug)]
pub struct Transaction {
    pub version: u32,
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
    pub lock_time: u32,
}

impl Transaction {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        // buffer.extend(self.version.to_le_bytes());
        // buffer.extend(self.inputs.len().to_varint_bytes());
        // for input in &self.inputs {
        //     buffer.extend(input.serialize());
        // }
        // buffer.extend(self.outputs.len().to_varint_bytes());
        // for output in &self.outputs {
        //     buffer.extend(output.serialize());
        // }
        // buffer.extend(self.lock_time.to_le_bytes());
        buffer
    }
    //pub fn parse(buffer: Vec<u8>) -> Result<Self, CustomError> {
    //let mut parser = BufferParser::new(buffer);
    pub fn parse(parser: &mut BufferParser) -> Result<Self, CustomError> {
        let version = parser.extract_u32()?;
        //chequear lo del flag
        let tx_in_count = parser.extract_varint()? as usize;
        let mut inputs = vec![];
        for input in 0..tx_in_count {
            // let ppio_de_input = parser.get_pos();
            // let prev_in = parser.move_i(36);
            // let script_sig_length = parser.extract_varint()? as usize;
            // let script_sig = parser.move_i(script_sig_length);
            // let sequence = parser.extract_u32()? as usize;
            // let tx_input_size = parser.get_pos() - ppio_de_input;
            inputs.push(TransactionInput::parse(parser)?);
        }
        let tx_out_count = parser.extract_varint()? as usize;
        let mut outputs = vec![];
        for output in 0..tx_out_count {
            // let ppio_de_output = parser.get_pos();
            // let value = parser.move_i(8);
            // let script_pubkey_length = parser.extract_varint()? as usize;
            // let script_pubkey = parser.move_i(script_pubkey_length);
            // let tx_output_size = parser.get_pos() - ppio_de_output;
            outputs.push(TransactionOutput::parse(parser)?);
        }

        let lock_time = parser.extract_u32()?;
        Ok(Self {
            version,
            inputs,
            outputs,
            lock_time,
        })
    }
}

#[derive(Debug)]
pub struct TransactionInput {
    pub previous_output: OutPoint,
    pub script_sig: Vec<u8>,
    pub sequence: u32,
}

impl TransactionInput {
    //pub fn parse(buffer: Vec<u8>) -> Result<Self, CustomError> {
    //let mut parser = BufferParser::new(buffer);
    pub fn parse(parser: &mut BufferParser) -> Result<Self, CustomError> {
        let previous_output = OutPoint::parse(parser.extract_buffer(36)?.to_vec())?;
        let script_sig_length = parser.extract_varint()? as usize;
        let script_sig = parser.extract_buffer(script_sig_length)?.to_vec();
        let sequence = parser.extract_u32()?;
        Ok(Self {
            previous_output,
            script_sig,
            sequence,
        })
    }
}

#[derive(Debug)]
pub struct OutPoint {
    pub hash: Vec<u8>,
    pub index: u32,
}

impl OutPoint {
    pub fn parse(buffer: Vec<u8>) -> Result<Self, CustomError> {
        let mut parser = BufferParser::new(buffer);
        let hash = parser.extract_buffer(32)?.to_vec();
        let index = parser.extract_u32()?;
        Ok(Self { hash, index })
    }
}

#[derive(Debug)]
pub struct TransactionOutput {
    pub value: u64,
    pub script_pubkey: Vec<u8>,
}

impl TransactionOutput {
    //pub fn parse(buffer: Vec<u8>) -> Result<Self, CustomError> {
    //let mut parser = BufferParser::new(buffer);
    pub fn parse(parser: &mut BufferParser) -> Result<Self, CustomError> {
        let value = parser.extract_u64()?;
        let script_pk_length = parser.extract_varint()? as usize;
        let script_pubkey = parser.extract_buffer(script_pk_length)?.to_vec();
        Ok(Self {
            value,
            script_pubkey,
        })
    }
}
