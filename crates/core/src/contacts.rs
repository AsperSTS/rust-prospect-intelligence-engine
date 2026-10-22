use regex::Regex;
use std::sync::LazyLock;
use url::Url;

static EMAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-z0-9._%+-]+@[a-z0-9-]+(?:\.[a-z0-9-]+)*\.[a-z]{2,}$").unwrap()
});
pub fn phone(raw: &str) -> String {
    let mut digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    for prefix in ["0052", "52", "044", "045", "01"] {
        if digits.starts_with(prefix) && digits.len() > 10 {
            digits = digits[prefix.len()..].to_owned();
            break;
        }
    }
    if digits.len() == 10 {
        digits
    } else {
        String::new()
    }
}
pub fn pretty_phone(raw: &str) -> String {
    let n = phone(raw);
    if n.is_empty() {
        raw.trim().to_owned()
    } else {
        format!("{} {} {}", &n[..3], &n[3..6], &n[6..])
    }
}
pub fn email_valid(raw: &str) -> bool {
    let e = raw.trim().to_lowercase();
    EMAIL.is_match(&e)
        && ![
            ".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".css", ".js",
        ]
        .iter()
        .any(|x| e.ends_with(x))
        && ![
            "ejemplo",
            "example",
            "correo.com",
            "email.com",
            "tudominio",
            "sentry",
            "wixpress",
            "domain.com",
            "sitio.com",
            "tuempresa",
            "yourdomain",
            "nombre@",
            "usuario@",
            "godaddy",
            "@2x",
            "noreply",
            "no-reply",
        ]
        .iter()
        .any(|x| e.contains(x))
}
pub fn normalize_url(raw: &str) -> Option<Url> {
    let s = raw.trim();
    if s.is_empty() || s.chars().any(char::is_whitespace) {
        return None;
    }
    let parsed = Url::parse(if s.contains("://") {
        s
    } else {
        return Url::parse(&format!("http://{s}")).ok().filter(safe_url);
    });
    parsed.ok().filter(safe_url)
}
pub fn safe_url(u: &Url) -> bool {
    matches!(u.scheme(), "http" | "https")
        && u.host_str().is_some()
        && u.username().is_empty()
        && u.password().is_none()
}
pub fn domain(raw: &str) -> String {
    let Some(u) = normalize_url(raw) else {
        return String::new();
    };
    let host = u
        .host_str()
        .unwrap_or("")
        .trim_start_matches("www.")
        .to_lowercase();
    let generics = [
        "facebook.com",
        "instagram.com",
        "linktr.ee",
        "wa.me",
        "google.com",
        "sites.google.com",
        "linkedin.com",
        "twitter.com",
        "x.com",
        "youtube.com",
        "tiktok.com",
        "wixsite.com",
        "wordpress.com",
        "blogspot.com",
        "weebly.com",
        "godaddysites.com",
        "mercadolibre.com.mx",
        "gmail.com",
        "hotmail.com",
        "outlook.com",
        "yahoo.com",
        "yahoo.com.mx",
        "prodigy.net.mx",
        "live.com",
        "icloud.com",
    ];
    if !host.contains('.')
        || host.parse::<std::net::IpAddr>().is_ok()
        || generics
            .iter()
            .any(|g| host == *g || host.ends_with(&format!(".{g}")))
    {
        String::new()
    } else {
        host
    }
}
pub fn whatsapp_url(raw: &str, text: &str) -> String {
    let n = phone(raw);
    if n.is_empty() {
        return String::new();
    }
    let mut u = Url::parse(&format!("https://wa.me/52{n}")).unwrap();
    u.query_pairs_mut().append_pair("text", text);
    u.into()
}
pub fn mailto(email: &str, subject: &str, body: &str) -> String {
    if !email_valid(email) {
        return String::new();
    }
    let mut u = Url::parse(&format!("mailto:{}", email.trim())).unwrap();
    u.query_pairs_mut()
        .append_pair("subject", subject)
        .append_pair("body", body);
    u.into()
}
