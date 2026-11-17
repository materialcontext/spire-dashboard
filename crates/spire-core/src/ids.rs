//! Typed identifiers. Every catalog entity is keyed by a string id from the
//! data source (e.g. `STRIKE_IRONCLAD`); wrapping them keeps a `CardId` from
//! being passed where a `RelicId` is expected.

use serde::{Deserialize, Serialize};

macro_rules! id_types {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {$(
        $(#[$doc])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(id: &str) -> Self {
                Self::new(id)
            }
        }

        impl From<String> for $name {
            fn from(id: String) -> Self {
                Self(id)
            }
        }
    )*};
}

id_types!(CardId, CharacterId, RelicId, PotionId, EnchantmentId, MonsterId, EncounterId, EventId, ActId, OrbId,);
