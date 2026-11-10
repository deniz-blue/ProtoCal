use twilight_http::{api_error::ApiError, error::ErrorType};

// Discord's JSON error codes.
pub const UNKNOWN_ROLE: u64 = 10011;
pub const CANNOT_SEND_MESSAGES_TO_THIS_USER: u64 = 50007;
pub const INVALID_FORM_BODY: u64 = 50035;

pub fn discord_error_code(error: &anyhow::Error) -> Option<u64> {
    let error = error.downcast_ref::<twilight_http::Error>()?;
    match error.kind() {
        ErrorType::Response { error: ApiError::General(general), .. } => Some(general.code),
        _ => None,
    }
}

pub fn is_discord_error(error: &anyhow::Error, code: u64) -> bool {
    discord_error_code(error) == Some(code)
}
