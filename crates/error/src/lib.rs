mod error;

pub use crate::error::Error;

#[cfg(test)]
mod tests {
    use crate::Error;

    #[test]
    fn test_error_code() {
        let err = Error::BadRequest(String::new());
        assert_eq!(err.code(), 400);

        let err = Error::Unauthorized(String::new());
        assert_eq!(err.code(), 401);

        let err = Error::NotFound(String::new());
        assert_eq!(err.code(), 404);

        let err = Error::Gone(String::new());
        assert_eq!(err.code(), 410);

        let err = Error::ImATeaPot(String::new());
        assert_eq!(err.code(), 418);

        let err = Error::ServerError(String::new());
        assert_eq!(err.code(), 500);

        let err = Error::BadGateway(String::new());
        assert_eq!(err.code(), 502);

        let err = Error::ServiceUnavailable(String::new());
        assert_eq!(err.code(), 503);
    }

    #[test]
    fn parsing_anyhow_error() {
        let generic_anyhow_err = anyhow::anyhow!("An error occurred");
        let err: Error = generic_anyhow_err.into();
        assert_eq!(err.code(), 418);
        assert_eq!(err.description(), "An error occurred".to_string());

        let compatible_err = anyhow::anyhow!(Error::NotFound("Item not found".to_string()));
        let err: Error = compatible_err.into();
        assert_eq!(err.code(), 404);
        assert_eq!(err.description(), "Item not found".to_string());
    }

    #[test]
    fn from_serdejson_error() {
        let serde_result: Result<serde_json::Value, serde_json::Error> =
            serde_json::from_str("invalid json");
        let serde_err = serde_result.unwrap_err();
        let err: Error = serde_err.into();
        assert_eq!(err.code(), 400);
        assert_eq!(err.description(), "expected value at line 1 column 1".to_string());
    }

    #[test]
    fn from_string() {
        let json_raw = r#"{"code":400,"description":"Bad Request"}"#.to_string();
        let err = Error::from_string(json_raw);
        assert_eq!(err.code(), 400);
        assert_eq!(err.description(), "Bad Request".to_string());

        let json_incompatible_raw = r#"{"status": 500, "message":"Error Occurred"}"#.to_string();
        let err = Error::from_string(json_incompatible_raw.clone());
        assert_eq!(err.code(), 418);
        assert_eq!(err.description(), json_incompatible_raw);

        let not_json_raw = "Some random error".to_string();
        let err = Error::from_string(not_json_raw);
        assert_eq!(err.code(), 418);
        assert_eq!(err.description(), "Some random error".to_string());
    }
}
