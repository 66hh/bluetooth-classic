pub fn mac_u64_to_string(addr: u64) -> String {
    let bytes = Vec::from(addr.to_be_bytes());

    let mac_parts: Vec<String> = bytes
        .iter()
        .skip(2)
        .take(6)
        .map(|b| format!("{:02X}", b))
        .collect();

    mac_parts.join(":")
}

pub fn mac_string_to_u64(addr: &String) -> Option<u64> {
    let cleaned = addr.split(':').collect::<Vec<_>>().join("");
    if cleaned.len() != 12 {
        return None;
    }

    match u64::from_str_radix(&cleaned, 16) {
        Ok(value) => return Some(value),
        Err(_) => return None,
    }
}
