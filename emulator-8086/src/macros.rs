/// Macro for constructing various jump instructions followed by IP-INC-8
#[macro_export]
macro_rules! jump_ipinc8_op {
    ($mnemonic:path, $value:expr ) => {
        OpcodeContext {
            first_byte_raw: $value,
            mnemonic: $mnemonic,
            next_field: NextFieldType::IpInc8,
            d: None,
            w: None,
            s: None,
            reg: None,
        }
    };
}
