use hex;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub struct TokenPair {
	pub plain: String,
	pub hash: String,
}

impl TokenPair {
	pub fn generate() -> Self {
		let plain = Uuid::new_v4().to_string();
		let hash = Self::hash(&plain);

		Self { plain, hash }
	}

	pub fn hash(token: &str) -> String {
		let mut hasher = Sha256::new();
		hasher.update(token.as_bytes());
		hex::encode(hasher.finalize())
	}
}
