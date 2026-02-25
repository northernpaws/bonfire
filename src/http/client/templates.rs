use askama::Template;

/// Provides the authorization URL for logging in with an OAuth provider.
pub struct OAuth2Provider {
    /// Internal ID for the provider.
    pub id: String,
    /// Name of the provider to show to the user.
    pub label: String,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub oauth2_providers: Vec<OAuth2Provider>,
}
