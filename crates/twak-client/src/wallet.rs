use common::Address;

pub fn validate_wallet(address: &Address) -> bool {
    address.looks_valid()
}
