use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EmptyExtraTokenFields,
    EndpointNotSet, EndpointSet, RedirectUrl, RevocationErrorResponseType, Scope,
    StandardErrorResponse, StandardRevocableToken, StandardTokenIntrospectionResponse,
    StandardTokenResponse, TokenResponse, TokenUrl,
    basic::{BasicClient, BasicErrorResponseType, BasicTokenType},
    reqwest,
};

use crate::{server::auth, user::UserId};

/// Configures an OAuth2 client that can be used for configuration.
#[derive(Clone)]
pub struct OauthClient {
    /// Provider ID used in lcoal application URLs.
    pub id: String,
    /// OAuth2 application client ID.
    pub client_id: String,
    /// OAuth2 application client secret.
    pub client_secret: String,
    /// OAuth2 application auth URI.
    pub auth_url: String,
    /// OAuth2 application token URI.
    pub token_url: String,
    /// Additional OAuth2 scopes that the application
    /// should request from the OAuth2 server.
    ///
    /// These may be required in situations where the
    /// server doesn't provide a field, such as email,
    /// without a custom scope.
    pub scopes: Vec<String>,
}

pub type OAuth2Client = oauth2::Client<
    StandardErrorResponse<BasicErrorResponseType>,
    StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>,
    StandardTokenIntrospectionResponse<EmptyExtraTokenFields, BasicTokenType>,
    StandardRevocableToken,
    StandardErrorResponse<RevocationErrorResponseType>,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

impl OauthClient {
    /// Builds an OAuth2 client using the parameters
    /// specified in the config struct.
    ///
    /// Public-facing client consumers should call `set_redirect_uri`
    /// to set the authorization redirect URL to a public endpoint.
    pub fn outh2_client(&self) -> OAuth2Client {
        let auth_url =
            AuthUrl::new(self.auth_url.clone()).expect("Invalid authorization endpoint URL");
        let token_url = TokenUrl::new(self.token_url.clone()).expect("Invalid token endpoint URL");

        BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_client_secret(ClientSecret::new(self.client_secret.clone()))
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
    }
}

#[derive(Clone)]
pub struct AuthConfig {
    /// OAuth2 clients that can be used by users to authenticate with SSO.
    pub oauth2_clients: Vec<OauthClient>,
}

pub struct AuthService {
    config: AuthConfig,
}

impl AuthService {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    /// Validates the supplied authentication token.
    pub fn validate_token(&self, token: &String) -> Option<UserId> {
        None
    }

    /// Generate an oauth2 authorization URL for the specified provider.
    pub fn oauth2_authorize_web(&self, provider: String, redirect_url: &String) -> Option<String> {
        // Parse and validate the supplied redirect URL.
        let Ok(redirect_url) = RedirectUrl::new("http://localhost:8080".to_string()) else {
            tracing::error!("invalid redirect url: {}", redirect_url);
            return None;
        };

        // Check that the specified provider is configured, and retrieve it's config.
        let Some(provider) = self.config.oauth2_clients.iter().find(|c| c.id == provider) else {
            tracing::error!("requested oauth provider {} not found", provider);
            return None;
        };

        // Build an `oauth2` client from the provider config.
        let client = provider.outh2_client().set_redirect_uri(
            RedirectUrl::new("http://localhost:8080".to_string()).expect("Invalid redirect URL"),
        );

        // Generate the authorization URL to redirect the user to;
        let (authorize_url, csrf_state) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("user:email".to_string()))
            // Add any custom scopes defined in the provider config by the admin.
            .add_scopes(provider.scopes.clone().into_iter().map(|s| Scope::new(s)))
            .url();

        // TODO: store the state token

        Some(authorize_url.to_string())
    }

    /// Exchange an oauth2 code and state for a token, and issue the user a local authentication token.
    pub fn oauth2_code_exchange_web(
        &self,
        provider: String,
        code: AuthorizationCode,
        state: CsrfToken,
    ) -> Option<String> {
        // Check that the specified provider is configured, and retrieve it's config.
        let Some(provider) = self.config.oauth2_clients.iter().find(|c| c.id == provider) else {
            tracing::error!("requested oauth provider {} not found", provider);
            return None;
        };

        // Build an `oauth2` client from the provider config.
        let client = provider.outh2_client().set_redirect_uri(
            RedirectUrl::new("http://localhost:8080".to_string()).expect("Invalid redirect URL"),
        );

        // Construct the HTTP client to use to exchange the code for a token.
        let http_client = reqwest::blocking::ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Client should build");

        // Exchange the code for an authorization token.
        let token = match client.exchange_code(code).request(&http_client) {
            Ok(token) => token,
            Err(err) => {
                tracing::error!(%err, "failed to exchange oauth2 code for token");
                return None;
            }
        };

        tracing::info!("OAuth2 code successfully exchanged for a token");

        // NB: Github returns a single comma-separated "scope" parameter instead of multiple
        // space-separated scopes. Github-specific clients can parse this scope into
        // multiple scopes by splitting at the commas. Note that it's not safe for the
        // library to do this by default because RFC 6749 allows scopes to contain commas.
        let scopes = if let Some(scopes_vec) = token.scopes() {
            scopes_vec
                .iter()
                .flat_map(|comma_separated| comma_separated.split(','))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        tracing::debug!("OAuth2 provider returned the following scopes: {scopes:?}");

        todo!("generate token");

        Some("".to_string())
    }
}
