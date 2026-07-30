#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

use core::{fmt, str::FromStr};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

/// Stable URL-safe alphabet used by [`StableId`].
///
/// The alphabet has exactly 64 symbols, so every character carries six bits.
/// Twenty-one characters encode the identifier's 126 bits.
pub const STABLE_ID_ALPHABET: &[u8; 64] =
    b"_-0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Length of the canonical NanoID-style text representation.
pub const STABLE_ID_TEXT_LEN: usize = 21;

/// Maximum integer representable by [`StableId`].
pub const STABLE_ID_MAX: u128 = (1_u128 << 126) - 1;

const DERIVE_DOMAIN: &[u8] = b"bevy_stable_id.derive.v1:";

/// Compact 126-bit persistent identity with a fixed 21-character text form.
///
/// `StableId` identifies one durable instance. It is distinct from an ECS
/// `Entity`, a Rust `TypeId`, and a semantic `Kind`.
#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableId(u128);

impl StableId {
    /// Sentinel value used when an optional ID must be represented in-band.
    pub const NIL: Self = Self(0);

    /// Construct an ID from its integer representation.
    pub const fn from_u128(value: u128) -> Result<Self, StableIdValueError> {
        if value <= STABLE_ID_MAX {
            Ok(Self(value))
        } else {
            Err(StableIdValueError { value })
        }
    }

    /// Return the stable integer representation.
    #[must_use]
    pub const fn as_u128(self) -> u128 {
        self.0
    }

    /// Generate a non-nil random identifier.
    #[cfg(feature = "random")]
    #[must_use]
    pub fn random() -> Self {
        loop {
            let value = rand::random::<u128>() & STABLE_ID_MAX;
            if value != 0 {
                return Self(value);
            }
        }
    }

    /// Deterministically derive a non-nil ID from a namespace and stable bytes.
    ///
    /// The result is stable across processes and platforms. Changing the
    /// namespace or any input byte produces a different identity.
    #[must_use]
    pub fn derive(namespace: Self, stable_bytes: impl AsRef<[u8]>) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(DERIVE_DOMAIN);
        hasher.update(&namespace.0.to_le_bytes());
        hasher.update(stable_bytes.as_ref());
        let digest = hasher.finalize();
        let mut raw = [0_u8; 16];
        raw.copy_from_slice(&digest.as_bytes()[..16]);
        let value = u128::from_le_bytes(raw) & STABLE_ID_MAX;
        Self(if value == 0 { 1 } else { value })
    }

    #[must_use]
    pub const fn is_nil(self) -> bool {
        self.0 == 0
    }

    /// Encode this ID using the canonical fixed-width URL-safe text form.
    #[must_use]
    pub fn to_nanoid(self) -> String {
        let mut value = self.0;
        let mut bytes = [0_u8; STABLE_ID_TEXT_LEN];

        for slot in bytes.iter_mut().rev() {
            *slot = STABLE_ID_ALPHABET[(value & 0x3f) as usize];
            value >>= 6;
        }

        String::from_utf8(bytes.to_vec()).expect("stable ID alphabet must remain valid UTF-8")
    }

    /// Decode the canonical fixed-width URL-safe text form.
    pub fn from_nanoid(value: &str) -> Result<Self, StableIdParseError> {
        if value.len() != STABLE_ID_TEXT_LEN {
            return Err(StableIdParseError::InvalidLength {
                expected: STABLE_ID_TEXT_LEN,
                actual: value.len(),
            });
        }

        let mut decoded = 0_u128;
        for (index, byte) in value.bytes().enumerate() {
            let symbol = STABLE_ID_ALPHABET
                .iter()
                .position(|candidate| *candidate == byte)
                .ok_or(StableIdParseError::InvalidCharacter {
                    index,
                    character: byte as char,
                })? as u128;

            decoded = (decoded << 6) | symbol;
        }

        Ok(Self(decoded))
    }
}

impl TryFrom<u128> for StableId {
    type Error = StableIdValueError;

    fn try_from(value: u128) -> Result<Self, Self::Error> {
        Self::from_u128(value)
    }
}

impl From<StableId> for u128 {
    fn from(value: StableId) -> Self {
        value.as_u128()
    }
}

impl fmt::Debug for StableId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("StableId")
            .field(&self.to_nanoid())
            .finish()
    }
}

impl fmt::Display for StableId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_nanoid())
    }
}

impl FromStr for StableId {
    type Err = StableIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_nanoid(value)
    }
}

#[cfg(feature = "serde")]
impl Serialize for StableId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_nanoid())
        } else {
            serializer.serialize_u128(self.0)
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for StableId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let value = String::deserialize(deserializer)?;
            Self::from_nanoid(&value).map_err(D::Error::custom)
        } else {
            let value = u128::deserialize(deserializer)?;
            Self::from_u128(value).map_err(D::Error::custom)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("stable ID integer exceeds the supported 126-bit range: {value}")]
pub struct StableIdValueError {
    pub value: u128,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StableIdParseError {
    #[error("stable ID must contain exactly {expected} characters, found {actual}")]
    InvalidLength { expected: usize, actual: usize },

    #[error("invalid stable ID character '{character}' at byte index {index}")]
    InvalidCharacter { index: usize, character: char },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_and_text_forms_round_trip() {
        for value in [0, 1, 42, (1_u128 << 64) + 9, STABLE_ID_MAX] {
            let id = StableId::from_u128(value).unwrap();
            let text = id.to_string();
            assert_eq!(text.len(), STABLE_ID_TEXT_LEN);
            assert_eq!(text.parse::<StableId>().unwrap(), id);
            assert_eq!(id.as_u128(), value);
        }
        assert!(StableId::from_u128(STABLE_ID_MAX + 1).is_err());
    }

    #[test]
    fn deterministic_derivation_is_namespaced() {
        let first = StableId::derive(StableId::NIL, b"settlement/market");
        let repeated = StableId::derive(StableId::NIL, b"settlement/market");
        let other_name = StableId::derive(StableId::NIL, b"settlement/harbor");
        let other_namespace =
            StableId::derive(StableId::from_u128(7).unwrap(), b"settlement/market");

        assert_eq!(first, repeated);
        assert!(!first.is_nil());
        assert_ne!(first, other_name);
        assert_ne!(first, other_namespace);
    }

    #[cfg(feature = "random")]
    #[test]
    fn random_ids_are_non_nil_and_round_trip() {
        for _ in 0..128 {
            let id = StableId::random();
            assert!(!id.is_nil());
            assert_eq!(id.to_string().parse::<StableId>().unwrap(), id);
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_uses_text_for_humans_and_integer_for_binary() {
        let id = StableId::derive(StableId::NIL, b"serde-contract");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(serde_json::from_str::<StableId>(&json).unwrap(), id);

        let bytes = postcard::to_stdvec(&id).unwrap();
        assert_eq!(postcard::from_bytes::<StableId>(&bytes).unwrap(), id);
    }
}
