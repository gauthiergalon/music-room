//! Input normalisation shared by every handler that accepts user identity fields.

use crate::errors::{AppError, ErrorMessage};

pub const USERNAME_MIN_CHARS: usize = 3;
pub const USERNAME_MAX_CHARS: usize = 24;
pub const PASSWORD_MIN_CHARS: usize = 8;
pub const PASSWORD_MAX_CHARS: usize = 128;
pub const MAX_FAVORITE_GENRES: usize = 20;
pub const GENRE_MAX_CHARS: usize = 32;

/// Trims the username and checks its length in characters (the column is `VARCHAR(24)`).
pub fn username(raw: &str) -> Result<String, ErrorMessage> {
    let trimmed = raw.trim();
    let length = trimmed.chars().count();

    if !(USERNAME_MIN_CHARS..=USERNAME_MAX_CHARS).contains(&length) {
        return Err(ErrorMessage::UsernameInvalidLength);
    }
    if trimmed.chars().any(char::is_control) {
        return Err(ErrorMessage::UsernameInvalidCharacters);
    }

    Ok(trimmed.to_string())
}

/// Trims and lowercases the address so lookups and the UNIQUE constraint agree.
pub fn email(raw: &str) -> Result<String, ErrorMessage> {
    let normalized = raw.trim().to_lowercase();

    if !validator::ValidateEmail::validate_email(&normalized) {
        return Err(ErrorMessage::EmailInvalidFormat);
    }

    Ok(normalized)
}

pub fn password(raw: &str) -> Result<(), ErrorMessage> {
    let length = raw.chars().count();

    if !(PASSWORD_MIN_CHARS..=PASSWORD_MAX_CHARS).contains(&length) {
        return Err(ErrorMessage::PasswordInvalidPolicy);
    }

    Ok(())
}

/// Trims the genres, drops empty and repeated ones, and bounds what a user
/// can store on their profile: free text, but not an unlimited amount of it.
pub fn favorite_genres(raw: Vec<String>) -> Result<Vec<String>, ErrorMessage> {
    let mut genres: Vec<String> = Vec::new();

    for genre in raw {
        let genre = genre.trim();
        if genre.is_empty() {
            continue;
        }
        if genre.chars().count() > GENRE_MAX_CHARS || genre.chars().any(char::is_control) {
            return Err(ErrorMessage::FavoriteGenresInvalid);
        }
        if !genres.iter().any(|known| known.eq_ignore_ascii_case(genre)) {
            genres.push(genre.to_string());
        }
    }

    if genres.len() > MAX_FAVORITE_GENRES {
        return Err(ErrorMessage::FavoriteGenresInvalid);
    }

    Ok(genres)
}

/// Wraps a single validation failure in the error type handlers return.
pub fn invalid(message: ErrorMessage) -> AppError {
    AppError::Validation(vec![message])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_is_trimmed_before_the_length_check() {
        assert_eq!(username("  bob  ").unwrap(), "bob");
        assert_eq!(
            username("      ").unwrap_err(),
            ErrorMessage::UsernameInvalidLength
        );
        assert_eq!(
            username(" a ").unwrap_err(),
            ErrorMessage::UsernameInvalidLength
        );
    }

    #[test]
    fn username_length_is_counted_in_characters() {
        // 24 two-byte characters fit in VARCHAR(24) even though they are 48 bytes.
        assert!(username(&"é".repeat(24)).is_ok());
        assert_eq!(
            username(&"é".repeat(25)).unwrap_err(),
            ErrorMessage::UsernameInvalidLength
        );
    }

    #[test]
    fn username_rejects_control_characters() {
        assert_eq!(
            username("bo\nb").unwrap_err(),
            ErrorMessage::UsernameInvalidCharacters
        );
    }

    #[test]
    fn email_is_normalized() {
        assert_eq!(email("  Foo@Example.COM ").unwrap(), "foo@example.com");
        assert_eq!(
            email("not-an-email").unwrap_err(),
            ErrorMessage::EmailInvalidFormat
        );
    }

    #[test]
    fn password_bounds() {
        assert!(password("12345678").is_ok());
        assert!(password("1234567").is_err());
        assert!(password(&"a".repeat(128)).is_ok());
        assert!(password(&"a".repeat(129)).is_err());
    }

    #[test]
    fn favorite_genres_are_cleaned_up() {
        let genres = favorite_genres(vec![
            "  Rock ".to_string(),
            "".to_string(),
            "rock".to_string(),
            "Électro".to_string(),
        ])
        .unwrap();

        assert_eq!(genres, vec!["Rock", "Électro"]);
    }

    #[test]
    fn favorite_genres_are_bounded() {
        let many = |count: usize| (0..count).map(|i| format!("genre {i}")).collect::<Vec<_>>();

        assert!(favorite_genres(many(MAX_FAVORITE_GENRES)).is_ok());
        assert_eq!(
            favorite_genres(many(MAX_FAVORITE_GENRES + 1)).unwrap_err(),
            ErrorMessage::FavoriteGenresInvalid
        );

        assert!(favorite_genres(vec!["a".repeat(GENRE_MAX_CHARS)]).is_ok());
        assert_eq!(
            favorite_genres(vec!["a".repeat(GENRE_MAX_CHARS + 1)]).unwrap_err(),
            ErrorMessage::FavoriteGenresInvalid
        );
        assert!(favorite_genres(vec!["ro\nck".to_string()]).is_err());
    }
}
