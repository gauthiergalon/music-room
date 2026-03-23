use lettre::message::header::ContentType;
use lettre::{Message, SmtpTransport, Transport};

use crate::errors::{AppError, ErrorMessage};

pub struct Email {
	pub subject: String,
	pub body: String,
}

static SENDER: &str = "Music Room <noreply@musicroom.app>";

impl Email {
	pub fn new(subject: impl Into<String>, body: impl Into<String>) -> Self {
		Self { subject: subject.into(), body: body.into() }
	}

	pub fn for_password_reset(token: &str) -> Self {
		Self::new("Reset your password", format!("Click on this link to reset your password...\n\nmusicroom://reset-password?token={}", token))
	}

	pub fn for_email_confirmation(token: &str) -> Self {
		Self::new("Confirm your email address", format!("Please confirm your email address by clicking on this link...\n\nmusicroom://confirm-email?token={}", token))
	}

	pub fn send(&self, receiver: &str) -> Result<(), AppError> {
		let email = Message::builder().from(SENDER.parse().map_err(|_| AppError::Internal)?).to(receiver.parse().map_err(|_| AppError::Internal)?).subject(&self.subject).header(ContentType::TEXT_PLAIN).body(self.body.clone()).map_err(|_| AppError::Internal)?;

		let mailer = SmtpTransport::builder_dangerous("localhost").port(1025).build();

		match mailer.send(&email) {
			Ok(_) => Ok(()),
			Err(e) => Err(AppError::Internal),
		}
	}
}
