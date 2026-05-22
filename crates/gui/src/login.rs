use ecb::cipher::{BlockEncryptMut, KeyInit, block_padding::Pkcs7};
use shared::{AppError, ErrorCode, SourceSite};
use crawler::{JxemallLoginHttpClient, JxemallLoginInput, JxemallLoginResponse};
use sm4::Sm4;

const JXEMALL_PASSWORD_PREFIX: &str = "zcyFront::";
const JXEMALL_SM4_KEY_HEX: &str = "edbd2139d9a7766e0382a2e6f92e9113";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginForm {
    pub source: SourceSite,
    pub username: String,
    pub password: String,
    pub remember_me: bool,
}

impl Default for LoginForm {
    fn default() -> Self {
        Self {
            source: SourceSite::Jxemall,
            username: String::new(),
            password: String::new(),
            remember_me: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoginFieldErrors {
    pub username: Option<String>,
    pub password: Option<String>,
}

impl LoginFieldErrors {
    pub fn is_empty(&self) -> bool {
        self.username.is_none() && self.password.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginStatus {
    Idle,
    Editing,
    Validating,
    Submitting,
    Success,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginSuccess {
    pub credential_ref: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginRequest {
    pub source: SourceSite,
    pub username: String,
    pub password: String,
}

pub trait LoginApiClient {
    fn login(&self, request: &LoginRequest) -> Result<LoginSuccess, AppError>;
}

pub struct CrawlerLoginApiClient {
    jxemall_client: JxemallLoginHttpClient,
    cookie_header: Option<String>,
}

impl CrawlerLoginApiClient {
    pub fn new(
        jxemall_base_url: impl Into<String>,
        cookie_header: Option<String>,
    ) -> Result<Self, AppError> {
        let jxemall_client = JxemallLoginHttpClient::new(jxemall_base_url)?;
        Ok(Self {
            jxemall_client,
            cookie_header,
        })
    }
}

impl LoginApiClient for CrawlerLoginApiClient {
    fn login(&self, request: &LoginRequest) -> Result<LoginSuccess, AppError> {
        match request.source {
            SourceSite::Jxemall => {
                let password_value = normalize_jxemall_password_value(&request.password)?;
                let response = self.jxemall_client.login(&JxemallLoginInput {
                    username: request.username.clone(),
                    password_value,
                    cookie_header: self.cookie_header.clone(),
                })?;

                Ok(login_success_from_jxemall_response(&response))
            }
        }
    }
}

fn normalize_jxemall_password_value(password: &str) -> Result<String, AppError> {
    let trimmed = password.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "password is required",
        ));
    }

    if trimmed.starts_with(JXEMALL_PASSWORD_PREFIX) {
        return Ok(trimmed.to_string());
    }

    let encrypted = encrypt_jxemall_password_value(trimmed)?;
    Ok(format!("{JXEMALL_PASSWORD_PREFIX}{encrypted}"))
}

fn encrypt_jxemall_password_value(plain: &str) -> Result<String, AppError> {
    type Sm4EcbEncryptor = ecb::Encryptor<Sm4>;

    let key = hex::decode(JXEMALL_SM4_KEY_HEX).map_err(|err| {
        AppError::new(
            ErrorCode::Infrastructure,
            format!("failed to decode sm4 key: {err}"),
        )
    })?;

    let cipher = Sm4EcbEncryptor::new_from_slice(&key).map_err(|err| {
        AppError::new(
            ErrorCode::Infrastructure,
            format!("failed to initialize sm4 ecb encryptor: {err}"),
        )
    })?;

    let encrypted = cipher.encrypt_padded_vec_mut::<Pkcs7>(plain.as_bytes());
    Ok(hex::encode(encrypted))
}

fn login_success_from_jxemall_response(response: &JxemallLoginResponse) -> LoginSuccess {
    let credential_ref = if let Some(sso) = response.sso_session.as_ref() {
        format!("cred://jxemall/sso/{sso}")
    } else if let Some(user_id) = response.user_id {
        format!("cred://jxemall/user/{user_id}")
    } else {
        "cred://jxemall/default".to_string()
    };

    LoginSuccess {
        credential_ref,
        expires_at: None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginPageState {
    pub form: LoginForm,
    pub field_errors: LoginFieldErrors,
    pub status: LoginStatus,
    pub show_password: bool,
    pub banner_error: Option<String>,
    pub last_success: Option<LoginSuccess>,
}

impl Default for LoginPageState {
    fn default() -> Self {
        Self {
            form: LoginForm::default(),
            field_errors: LoginFieldErrors::default(),
            status: LoginStatus::Idle,
            show_password: false,
            banner_error: None,
            last_success: None,
        }
    }
}

impl LoginPageState {
    pub fn update_username(&mut self, username: impl Into<String>) {
        self.form.username = username.into();
        self.status = LoginStatus::Editing;
        self.field_errors.username = None;
        self.banner_error = None;
    }

    pub fn update_password(&mut self, password: impl Into<String>) {
        self.form.password = password.into();
        self.status = LoginStatus::Editing;
        self.field_errors.password = None;
        self.banner_error = None;
    }

    pub fn set_remember_me(&mut self, remember_me: bool) {
        self.form.remember_me = remember_me;
        self.status = LoginStatus::Editing;
    }

    pub fn toggle_password_visibility(&mut self) {
        self.show_password = !self.show_password;
    }

    pub fn validate(&mut self) -> Result<(), LoginFieldErrors> {
        self.status = LoginStatus::Validating;

        let errors = validate_login_form(&self.form);
        self.field_errors = errors.clone();

        if errors.is_empty() {
            Ok(())
        } else {
            self.status = LoginStatus::Failed;
            Err(errors)
        }
    }

    pub fn submit<C: LoginApiClient>(&mut self, client: &C) -> Result<LoginSuccess, AppError> {
        if let Err(_errors) = self.validate() {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "login form validation failed",
            ));
        }

        self.status = LoginStatus::Submitting;

        let request = LoginRequest {
            source: self.form.source.clone(),
            username: self.form.username.trim().to_string(),
            password: self.form.password.clone(),
        };

        match client.login(&request) {
            Ok(success) => {
                self.status = LoginStatus::Success;
                self.banner_error = None;
                self.last_success = Some(success.clone());
                Ok(success)
            }
            Err(err) => {
                self.status = LoginStatus::Failed;
                self.banner_error = Some(err.message.clone());
                Err(err)
            }
        }
    }
}

pub fn validate_login_form(form: &LoginForm) -> LoginFieldErrors {
    let username = form.username.trim();
    let password = form.password.trim();

    let mut errors = LoginFieldErrors::default();

    if username.is_empty() {
        errors.username = Some("请输入账号".to_string());
    }

    if password.is_empty() {
        errors.password = Some("请输入密码".to_string());
    }

    errors
}

#[cfg(test)]
mod tests {
    use shared::{AppError, ErrorCode};

    use super::{
        login_success_from_jxemall_response, normalize_jxemall_password_value, LoginApiClient,
        LoginPageState, LoginRequest, LoginStatus, LoginSuccess, validate_login_form,
    };
    use crawler::JxemallLoginResponse;

    struct LoginApiClientOk;

    impl LoginApiClient for LoginApiClientOk {
        fn login(&self, _request: &LoginRequest) -> Result<LoginSuccess, AppError> {
            Ok(LoginSuccess {
                credential_ref: "cred://jxemall/default".to_string(),
                expires_at: Some("2026-05-21T23:59:59Z".to_string()),
            })
        }
    }

    struct LoginApiClientErr;

    impl LoginApiClient for LoginApiClientErr {
        fn login(&self, _request: &LoginRequest) -> Result<LoginSuccess, AppError> {
            Err(AppError::new(
                ErrorCode::Unauthorized,
                "用户名或密码错误",
            ))
        }
    }

    #[test]
    fn validate_should_fail_when_username_and_password_are_empty() {
        let state = LoginPageState::default();
        let errors = validate_login_form(&state.form);

        assert_eq!(errors.username.as_deref(), Some("请输入账号"));
        assert_eq!(errors.password.as_deref(), Some("请输入密码"));
    }

    #[test]
    fn submit_should_fail_when_form_is_invalid() {
        let mut state = LoginPageState::default();
        let result = state.submit(&LoginApiClientOk);

        assert!(result.is_err());
        assert_eq!(state.status, LoginStatus::Failed);
        assert_eq!(state.field_errors.username.as_deref(), Some("请输入账号"));
        assert_eq!(state.field_errors.password.as_deref(), Some("请输入密码"));
    }

    #[test]
    fn submit_should_trim_username_and_succeed() {
        let mut state = LoginPageState::default();
        state.update_username("  demo-user  ");
        state.update_password("demo-pass");

        let result = state.submit(&LoginApiClientOk).expect("login should succeed");

        assert_eq!(result.credential_ref, "cred://jxemall/default");
        assert_eq!(state.status, LoginStatus::Success);
        assert!(state.banner_error.is_none());
        assert!(state.last_success.is_some());
    }

    #[test]
    fn submit_should_set_failed_status_when_api_returns_error() {
        let mut state = LoginPageState::default();
        state.update_username("demo-user");
        state.update_password("wrong-pass");

        let result = state.submit(&LoginApiClientErr);

        assert!(result.is_err());
        assert_eq!(state.status, LoginStatus::Failed);
        assert_eq!(state.banner_error.as_deref(), Some("用户名或密码错误"));
    }

    #[test]
    fn toggle_password_visibility_should_flip_boolean() {
        let mut state = LoginPageState::default();

        assert!(!state.show_password);
        state.toggle_password_visibility();
        assert!(state.show_password);
        state.toggle_password_visibility();
        assert!(!state.show_password);
    }

    #[test]
    fn should_map_jxemall_response_to_credential_ref_with_sso() {
        let response = JxemallLoginResponse {
            code: "0000".to_string(),
            success: true,
            message: "成功".to_string(),
            processing_url: Some("https://member.jxemall.com/login".to_string()),
            redirect: true,
            show_captcha: false,
            user_id: Some(10008679985),
            set_cookie_headers: vec![
                "SSOSESSION=abc123; Path=/; Secure; HttpOnly".to_string(),
            ],
            sso_session: Some("abc123".to_string()),
        };

        let mapped = login_success_from_jxemall_response(&response);
        assert_eq!(mapped.credential_ref, "cred://jxemall/sso/abc123");
        assert!(mapped.expires_at.is_none());
    }

    #[test]
    fn should_accept_password_value_with_zcy_prefix() {
        let value = normalize_jxemall_password_value(" zcyFront::abcdef ")
            .expect("prefixed password value should pass");
        assert_eq!(value, "zcyFront::abcdef");
    }

    #[test]
    fn should_encrypt_plaintext_password_value() {
        let value = normalize_jxemall_password_value("plain-text-password")
            .expect("plaintext should be auto encrypted");
        assert!(value.starts_with("zcyFront::"));
        assert_ne!(value, "zcyFront::plain-text-password");
    }
}
