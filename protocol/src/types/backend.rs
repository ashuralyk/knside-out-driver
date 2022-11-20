use ckb_types::packed::OutPoint;

pub enum KoRequestInput {
    Address(String),
    Outpoints(Vec<OutPoint>),
}
