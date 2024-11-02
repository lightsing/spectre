use alloy_primitives::bytes::{BufMut, BytesMut};
use alloy_primitives::utils::{ParseUnits, Unit};
use alloy_primitives::{utils::parse_units, Address, Bytes, U256};
use hex::FromHexError;
use serde::{Deserialize, Deserializer};
use std::fmt::{Debug, Formatter};
use std::str::FromStr;

#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct Ether(pub U256);
impl<'de> Deserialize<'de> for Ether {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // split into amount and unit
        let mut parts = s.split(' ');
        let parsed = parse_units(
            parts
                .next()
                .ok_or(serde::de::Error::custom("missing amount"))?,
            parts
                .next()
                .ok_or(serde::de::Error::custom("missing unit"))?,
        )
        .map_err(|e| serde::de::Error::custom(e))?;
        if parsed.is_negative() {
            return Err(serde::de::Error::custom("negative ether"));
        }
        Ok(Ether(parsed.get_absolute()))
    }
}

impl Debug for Ether {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        const UNITS: &[(Unit, &str, U256)] = &[
            (
                Unit::ETHER,
                "Ether",
                U256::from_limbs([1_000_000_000_000_000_000, 0, 0, 0]),
            ),
            (
                Unit::PWEI,
                "PWei",
                U256::from_limbs([1_000_000_000_000_000, 0, 0, 0]),
            ),
            (
                Unit::TWEI,
                "TWei",
                U256::from_limbs([1_000_000_000_000, 0, 0, 0]),
            ),
            (
                Unit::GWEI,
                "GWei",
                U256::from_limbs([1_000_000_000, 0, 0, 0]),
            ),
            (Unit::MWEI, "MWei", U256::from_limbs([1_000_000, 0, 0, 0])),
            (Unit::KWEI, "KWei", U256::from_limbs([1_000, 0, 0, 0])),
            (Unit::WEI, "Wei", U256::from_limbs([1, 0, 0, 0])),
        ];
        for (unit, literal, value) in UNITS {
            if self.0 >= *value {
                return write!(
                    f,
                    "{} {literal}",
                    ParseUnits::U256(self.0).format_units(*unit)
                );
            }
        }
        write!(
            f,
            "{} wei",
            ParseUnits::U256(self.0).format_units(Unit::WEI)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressOrAlias {
    Address(Address),
    Alias(String),
}

impl<'de> Deserialize<'de> for AddressOrAlias {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.starts_with("0x") {
            Ok(AddressOrAlias::Address(
                Address::from_str(s.as_str()).map_err(serde::de::Error::custom)?,
            ))
        } else {
            Ok(AddressOrAlias::Alias(s.to_string()))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("invalid opcode {opcode} at line {line}")]
    InvalidOpcode { opcode: String, line: usize },
    #[error("missing value for opcode {opcode} at line {line}")]
    MissingValue { opcode: String, line: usize },
    #[error("invalid push value hex at line {line}: {error}")]
    InvalidPushValueHex { line: usize, error: FromHexError },
    #[error("invalid push value length at line {line}: expected {expected}, got {length}")]
    InvalidPushValueLength {
        line: usize,
        expected: usize,
        length: usize,
    },
}

pub fn compile_mnemonic(codes: &str) -> Result<Bytes, CompileError> {
    let mut code = BytesMut::new();
    for (idx, line) in codes.split('\n').enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut line = line.split_whitespace();
        let opcode = line.next().unwrap().to_ascii_uppercase();
        match opcode.as_str() {
            "STOP" => code.put_u8(0x00),
            "ADD" => code.put_u8(0x01),
            "MUL" => code.put_u8(0x02),
            "SUB" => code.put_u8(0x03),
            "DIV" => code.put_u8(0x04),
            "SDIV" => code.put_u8(0x05),
            "MOD" => code.put_u8(0x06),
            "SMOD" => code.put_u8(0x07),
            "ADDMOD" => code.put_u8(0x08),
            "MULMOD" => code.put_u8(0x09),
            "EXP" => code.put_u8(0x0a),
            "SIGNEXTEND" => code.put_u8(0x0b),
            "LT" => code.put_u8(0x10),
            "GT" => code.put_u8(0x11),
            "SLT" => code.put_u8(0x12),
            "SGT" => code.put_u8(0x13),
            "EQ" => code.put_u8(0x14),
            "ISZERO" => code.put_u8(0x15),
            "AND" => code.put_u8(0x16),
            "OR" => code.put_u8(0x17),
            "XOR" => code.put_u8(0x18),
            "NOT" => code.put_u8(0x19),
            "BYTE" => code.put_u8(0x1a),
            "SHL" => code.put_u8(0x1b),
            "SHR" => code.put_u8(0x1c),
            "SAR" => code.put_u8(0x1d),
            "SHA3" => code.put_u8(0x20),
            "ADDRESS" => code.put_u8(0x30),
            "BALANCE" => code.put_u8(0x31),
            "ORIGIN" => code.put_u8(0x32),
            "CALLER" => code.put_u8(0x33),
            "CALLVALUE" => code.put_u8(0x34),
            "CALLDATALOAD" => code.put_u8(0x35),
            "CALLDATASIZE" => code.put_u8(0x36),
            "CALLDATACOPY" => code.put_u8(0x37),
            "CODESIZE" => code.put_u8(0x38),
            "CODECOPY" => code.put_u8(0x39),
            "GASPRICE" => code.put_u8(0x3a),
            "EXTCODESIZE" => code.put_u8(0x3b),
            "EXTCODECOPY" => code.put_u8(0x3c),
            "RETURNDATASIZE" => code.put_u8(0x3d),
            "RETURNDATACOPY" => code.put_u8(0x3e),
            "EXTCODEHASH" => code.put_u8(0x3f),
            "BLOCKHASH" => code.put_u8(0x40),
            "COINBASE" => code.put_u8(0x41),
            "TIMESTAMP" => code.put_u8(0x42),
            "NUMBER" => code.put_u8(0x43),
            "DIFFICULTY" => code.put_u8(0x44),
            "GASLIMIT" => code.put_u8(0x45),
            "CHAINID" => code.put_u8(0x46),
            "SELFBALANCE" => code.put_u8(0x47),
            "POP" => code.put_u8(0x50),
            "MLOAD" => code.put_u8(0x51),
            "MSTORE" => code.put_u8(0x52),
            "MSTORE8" => code.put_u8(0x53),
            "SLOAD" => code.put_u8(0x54),
            "SSTORE" => code.put_u8(0x55),
            "JUMP" => code.put_u8(0x56),
            "JUMPI" => code.put_u8(0x57),
            "PC" => code.put_u8(0x58),
            "MSIZE" => code.put_u8(0x59),
            "GAS" => code.put_u8(0x5a),
            "JUMPDEST" => code.put_u8(0x5b),
            "TLOAD" => code.put_u8(0x5c),
            "TSTORE" => code.put_u8(0x5d),
            _ if opcode.starts_with("PUSH") => {
                let n = opcode[4..].parse::<usize>().unwrap();
                code.put_u8(0x60 + n as u8);
                if n == 0 {
                    continue;
                }
                if n > 32 {
                    return Err(CompileError::InvalidOpcode { line: idx, opcode });
                }
                let value = line.next().ok_or_else(|| CompileError::MissingValue {
                    opcode: opcode.clone(),
                    line: idx,
                })?;
                let value = if value.starts_with("0x") {
                    hex::decode(&value[2..])
                } else {
                    hex::decode(value)
                }
                .map_err(|e| CompileError::InvalidPushValueHex {
                    line: idx,
                    error: e,
                })?;
                if value.len() > n {
                    return Err(CompileError::InvalidPushValueLength {
                        line: idx,
                        expected: n,
                        length: value.len(),
                    });
                }
                for _ in 0..n - value.len() {
                    code.put_u8(0);
                }
                code.extend(value);
            }
            _ if opcode.starts_with("DUP") => {
                let n = opcode[3..].parse::<u8>().unwrap();
                if n == 0 || n > 16 {
                    return Err(CompileError::InvalidOpcode { opcode, line: idx });
                }
                code.put_u8(0x7f + n);
            }
            _ if opcode.starts_with("SWAP") => {
                let n = opcode[4..].parse::<u8>().unwrap();
                if n == 0 || n > 16 {
                    return Err(CompileError::InvalidOpcode { opcode, line: idx });
                }
                code.put_u8(0x8f + n);
            }
            _ if opcode.starts_with("LOG") => {
                let n = opcode[3..].parse::<u8>().unwrap();
                if n > 4 {
                    return Err(CompileError::InvalidOpcode { opcode, line: idx });
                }
                code.put_u8(0xa0 + n);
            }
            "CREATE" => code.put_u8(0xf0),
            "CALL" => code.put_u8(0xf1),
            "CALLCODE" => code.put_u8(0xf2),
            "RETURN" => code.put_u8(0xf3),
            "DELEGATECALL" => code.put_u8(0xf4),
            "CREATE2" => code.put_u8(0xf5),
            "STATICCALL" => code.put_u8(0xfa),
            "REVERT" => code.put_u8(0xfd),
            "INVALID" => code.put_u8(0xfe),
            "SELFDESTRUCT" => code.put_u8(0xff),
            _ => return Err(CompileError::InvalidOpcode { opcode, line: idx }),
        }
    }
    Ok(Bytes::from(code.freeze()))
}

pub const fn default_true() -> bool {
    true
}
pub const fn default_false() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_mnemonic() {
        let code = compile_mnemonic(
            r#"
            PUSH1 0x60
            PUSH1 0x40
            MSTORE
            PUSH1 0x00
            RETURN
            "#,
        )
        .unwrap();
        assert_eq!(
            code,
            Bytes::from_static(&[0x61, 0x60, 0x61, 0x40, 0x52, 0x61, 0x00, 0xf3])
        );

        let code = compile_mnemonic(
            r#"
            PUSH0
            PUSH1 0x01
            DUP1
            SWAP1
            LOG0
            "#,
        )
        .unwrap();
        assert_eq!(
            code,
            Bytes::from_static(&[0x60, 0x61, 0x01, 0x80, 0x90, 0xa0])
        );
    }
}
