/// Keys whose values are replaced unless the operator opts into secrets.
const SECRET_NEEDLES: &[&str] = &[
    "PASSWORD",
    "PASSWD",
    "SECRET",
    "TOKEN",
    "APIKEY",
    "API_KEY",
    "PRIVATE",
    "CREDENTIAL",
    "AUTH",
    "ACCESS_KEY",
    "SECRET_KEY",
    "BEARER",
    "COOKIE",
    "SESSION",
];

pub fn is_secret_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    SECRET_NEEDLES.iter().any(|n| upper.contains(n))
}

pub fn redact_value(key: &str, value: &str, show_secrets: bool) -> String {
    if show_secrets || !is_secret_key(key) {
        value.to_string()
    } else {
        "********".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_common_secret_names() {
        assert!(is_secret_key("DB_PASSWORD"));
        assert!(is_secret_key("aws_secret_access_key"));
        assert!(!is_secret_key("HOME"));
        assert_eq!(redact_value("TOKEN", "abc", false), "********");
        assert_eq!(redact_value("TOKEN", "abc", true), "abc");
    }
}
