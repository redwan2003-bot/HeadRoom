//! Headroom Engine: Cleaning actions coordinator, cooldown enforcement, and receipt logging.

pub mod cooldown;
pub mod executor;
pub mod receipts;

pub use cooldown::CooldownManager;
pub use executor::{CleaningEngine, EngineError};
pub use receipts::{append_receipt, CleanReceipt, RECEIPT_SCHEMA_VERSION};
