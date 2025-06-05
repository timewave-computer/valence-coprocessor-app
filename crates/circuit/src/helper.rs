// we don't know exactly how much padding occurred
// therefore we decide based on the slot size of the string
pub fn truncate_neutron_address(mut data: Vec<u8>, is_receiver_contract: bool) -> Vec<u8> {
    if is_receiver_contract {
        // a 66 byte neutron address
        while data.last() == Some(&0x00) && data.len() > 66 {
            data.pop();
        }
    }
    // a 46 byte neutron address
    while data.last() == Some(&0x00) && data.len() > 46 {
        data.pop();
    }
    data
}
