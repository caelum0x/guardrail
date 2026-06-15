//! Converts natural-language mandates into validated machine policy artifacts.

pub mod compiler;
pub mod defaults;
pub mod parser;
pub mod policy_hash;
pub mod schema;
pub mod validator;

pub use compiler::{compile_json_policy, compile_mandate, CompiledPolicy};
pub use parser::{normalize_mandate, parse_mandate};
pub use policy_hash::policy_hash;
pub use validator::validate_policy;
