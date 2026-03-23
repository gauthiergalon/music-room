use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct User {
	pub id: Uuid,
	pub username: String,
	pub email: String,
	pub password_hash: Option<String>,
	pub google_id: Option<String>,
}

pub struct NewUser<'a> {
	pub username: &'a str,
	pub email: &'a str,
	pub password_hash: Option<String>,
	pub google_id: Option<String>,
}
