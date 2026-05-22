use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, COOKIE, ORIGIN, REFERER, SET_COOKIE};
use reqwest::blocking::multipart::{Form, Part};
use serde::Deserialize;
use shared::{AppError, ErrorCode};

use crate::{
    build_jxemall_login_form_fields, JxemallLoginFormInput, JXEMALL_LOGIN_ORIGIN,
    JXEMALL_LOGIN_PATH, JXEMALL_LOGIN_QUERY_CURRENT_URI, JXEMALL_LOGIN_REFERER,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JxemallLoginInput {
    pub username: String,
    pub password_value: String,
    pub cookie_header: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JxemallLoginResponse {
    pub code: String,
    pub success: bool,
    pub message: String,
    pub processing_url: Option<String>,
    pub redirect: bool,
    pub show_captcha: bool,
    pub user_id: Option<u64>,
    pub set_cookie_headers: Vec<String>,
    pub sso_session: Option<String>,
}

pub struct JxemallLoginHttpClient {
    client: Client,
    base_url: String,
}

impl JxemallLoginHttpClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self, AppError> {
        let client = Client::builder()
            .build()
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to build http client: {err}"),
                )
            })?;

        Ok(Self {
            client,
            base_url: base_url.into(),
        })
    }

    pub fn login(&self, input: &JxemallLoginInput) -> Result<JxemallLoginResponse, AppError> {
        let fields = build_jxemall_login_form_fields(&JxemallLoginFormInput {
            username: input.username.clone(),
            password_value: input.password_value.clone(),
        })?;

        let mut form = Form::new();
        for (name, value) in fields {
            form = form.part(name, Part::text(value));
        }

        let url = format!(
            "{base}{path}?{query}",
            base = self.base_url.trim_end_matches('/'),
            path = JXEMALL_LOGIN_PATH,
            query = JXEMALL_LOGIN_QUERY_CURRENT_URI,
        );

        let mut request = self
            .client
            .post(url)
            .header(ACCEPT, "application/json, text/plain, */*")
            .header(ORIGIN, JXEMALL_LOGIN_ORIGIN)
            .header(REFERER, JXEMALL_LOGIN_REFERER)
            .multipart(form);

        if let Some(cookie_header) = input.cookie_header.as_ref() {
            request = request.header(COOKIE, cookie_header);
        }

        let response = request.send().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("login request failed: {err}"),
            )
        })?;

        let status = response.status();
        let set_cookie_headers: Vec<String> = response
            .headers()
            .get_all(SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect();

        let body: LoginBody = response.json().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to parse login response json: {err}"),
            )
        })?;

        if !status.is_success() {
            return Err(AppError::new(
                ErrorCode::Unauthorized,
                format!("login failed with http status {status}: {}", body.message),
            ));
        }

        if body.code != "0000" || !body.success {
            return Err(AppError::new(
                ErrorCode::Unauthorized,
                format!("login rejected: {} ({})", body.message, body.code),
            ));
        }

        let sso_session = set_cookie_headers
            .iter()
            .find_map(|line| extract_cookie_value(line, "SSOSESSION"));

        Ok(JxemallLoginResponse {
            code: body.code,
            success: body.success,
            message: body.message,
            processing_url: body.data.as_ref().and_then(|d| d.processing_url.clone()),
            redirect: body.data.as_ref().map(|d| d.redirect).unwrap_or(false),
            show_captcha: body.data.as_ref().map(|d| d.show_captcha).unwrap_or(false),
            user_id: body.data.as_ref().and_then(|d| d.user_id),
            set_cookie_headers,
            sso_session,
        })
    }
}

#[derive(Debug, Deserialize)]
struct LoginBody {
    code: String,
    success: bool,
    message: String,
    data: Option<LoginData>,
}

#[derive(Debug, Deserialize)]
struct LoginData {
    #[serde(rename = "processingUrl")]
    processing_url: Option<String>,
    redirect: bool,
    #[serde(rename = "showCaptcha")]
    show_captcha: bool,
    #[serde(rename = "userId")]
    user_id: Option<u64>,
}

pub fn extract_cookie_value(set_cookie_line: &str, cookie_name: &str) -> Option<String> {
    let prefix = format!("{cookie_name}=");
    let first = set_cookie_line.split(';').next()?.trim();
    if !first.starts_with(&prefix) {
        return None;
    }

    Some(first[prefix.len()..].to_string())
}

#[cfg(test)]
mod tests {
    use super::extract_cookie_value;

    #[test]
    fn should_extract_cookie_value_from_set_cookie_line() {
        let line = "SSOSESSION=abc123xyz; Path=/; Secure; HttpOnly";
        let value = extract_cookie_value(line, "SSOSESSION");
        assert_eq!(value.as_deref(), Some("abc123xyz"));
    }

    #[test]
    fn should_return_none_when_cookie_name_does_not_match() {
        let line = "SESSION=aaa; Path=/";
        let value = extract_cookie_value(line, "SSOSESSION");
        assert!(value.is_none());
    }
}
