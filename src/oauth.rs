use leptos::{prelude::*, server_fn::codec::GetUrl};

#[cfg(feature = "ssr")]
pub mod oauth {
    use std::collections::HashMap;
    use std::sync::{Arc, LazyLock};

    use axum::extract::{FromRef, Request};

    use axum::middleware::Next;
    use axum::response::Response;
    use iddqd::{IdHashItem, IdHashMap, id_upcast};
    use leptos::prelude::ServerFnError;
    use leptos::{config::LeptosOptions, prelude::expect_context};
    use oauth_axum::providers::discord::DiscordProvider;
    use oauth_axum::{CustomProvider, OAuthClient};
    use serde::{Deserialize, Serialize};
    use surreal_derive_plus::SurrealDerive;
    use surreal_devl::surreal_id::SurrealId;
    use tokio::sync::{Mutex, RwLock};

    use crate::database::USERS;
    use crate::secrets::{Secrets, get_secrets};
    #[derive(Clone)]
    pub struct AppState {
        pub leptos_options: LeptosOptions,
        pub states: Arc<Mutex<HashMap<String, String>>>,
        pub users: Arc<RwLock<IdHashMap<User>>>,
    }

    pub static AUTH_TOKENS: LazyLock<RwLock<HashMap<String, u64>>> = LazyLock::new(RwLock::default);

    impl FromRef<AppState> for LeptosOptions {
        fn from_ref(input: &AppState) -> Self {
            input.leptos_options.clone()
        }
    }

    pub fn get_client(secrets: &Secrets) -> CustomProvider {
        DiscordProvider::new(
            secrets.discord_client_id.clone(),
            secrets.discord_client_secret.clone(),
            "http://localhost:3000/discord_callback".to_owned(),
        )
    }

    pub async fn create_url() -> Result<String, ServerFnError> {
        let state: AppState = expect_context();
        let secrets = get_secrets().await;
        let state_oauth = get_client(secrets)
            .generate_url(vec!["identify".to_owned()], |state_e| async move {
                state
                    .states
                    .lock()
                    .await
                    .insert(state_e.state, state_e.verifier);
            })
            .await
            .ok()
            .unwrap()
            .state
            .unwrap();

        state_oauth
            .url_generated
            .ok_or_else(|| ServerFnError::ServerError("Couldn't get auth".to_owned()))
    }

    #[derive(Clone, serde::Deserialize)]
    pub struct QueryAxumCallback {
        pub code: String,
        pub state: String,
    }

    pub async fn authentication_middleware(mut request: Request, next: Next) -> Response {
        use axum::RequestExt;
        use axum::{body::Body, http::StatusCode};

        let cookies: tower_cookies::Cookies = request.extract_parts().await.unwrap();
        let auth_tokens = AUTH_TOKENS.read().await;
        if request.uri().path().starts_with("/movies") {
            if cookies
                .get("token")
                .is_none_or(|t| !auth_tokens.contains_key(t.value()))
            {
                let mut response = Response::new(Body::empty());
                *response.status_mut() = StatusCode::from_u16(401).unwrap();
                return response;
            }
        }

        let response = next.run(request).await;

        response
    }

    #[derive(SurrealDerive, Serialize, Deserialize, Debug)]
    pub struct User {
        pub user_id: u64,
        pub name: String,
        pub auth_tokens: Vec<String>,
    }

    impl IdHashItem for User {
        type Key<'a> = u64;

        fn key(&self) -> Self::Key<'_> {
            self.user_id
        }

        id_upcast!();
    }

    use surrealdb::sql::Id;
    use surrealdb::sql::Thing;
    impl SurrealId for User {
        fn id(&self) -> Thing {
            Thing::from((USERS, Id::Number(self.user_id as i64)))
        }
    }
}
#[server (prefix="", endpoint="", input = GetUrl)]
pub async fn authenticate() -> Result<(), ServerFnError> {
    use crate::oauth::oauth::AUTH_TOKENS;
    use crate::oauth::oauth::create_url;
    use leptos_axum::extract;
    let cookies: tower_cookies::Cookies = extract().await?;
    if let Some(token) = cookies.get("token")
        && AUTH_TOKENS.read().await.contains_key(token.value())
    {
        leptos_axum::redirect("/movies");
        return Ok(());
    }
    leptos_axum::redirect(&create_url().await?);
    Ok(())
}

#[server (prefix="", endpoint="discord_callback", input = GetUrl)]
pub async fn discord_callback() -> Result<(), ServerFnError> {
    use crate::database::write_auth_token;
    use crate::oauth::oauth::AUTH_TOKENS;
    use crate::oauth::oauth::{AppState, QueryAxumCallback};
    use crate::secrets::get_secrets;
    use axum::extract::Query;
    use leptos::logging::log;
    use leptos_axum::extract;
    use oauth::get_client;
    use oauth_axum::OAuthClient;
    use oauth_axum::error::OauthError::{AuthUrlCreationFailed, TokenRequestFailed};
    use rand::Rng;
    use rand::distr::Alphanumeric;
    use serde::Deserialize;
    use serde_with::{DisplayFromStr, serde_as};
    use tower_cookies::{Cookie, cookie::time::Duration};
    #[serde_as]
    #[derive(Deserialize, Debug)]
    struct User {
        #[serde_as(as = "DisplayFromStr")]
        id: u64,
    }
    log!("GOT CALLBACK");
    let app_state: AppState = expect_context();
    let queries: Query<QueryAxumCallback> = extract().await?;
    let state = app_state
        .states
        .lock()
        .await
        .remove(&queries.state)
        .ok_or_else(|| ServerFnError::new("Failed to find state"))?;
    let secrets = get_secrets().await;
    let token = match get_client(secrets)
        .generate_token(queries.code.clone(), state)
        .await
    {
        Ok(p) => p,
        Err(e) => match e {
            TokenRequestFailed => panic!("Token request failed"),
            AuthUrlCreationFailed => panic!("auth url creation failed"),
        },
    };
    let client = reqwest::Client::new();
    let user: User = client
        .get("https://discord.com/api/users/@me")
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let token: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    write_auth_token(user.id, &token)
        .await
        .map_err(ServerFnError::new)?;
    // app_state
    //     .auth_tokens
    //     .write()
    //     .await
    //     .insert(token.clone(), user.id);
    AUTH_TOKENS.write().await.insert(token.clone(), user.id);
    let cookies: tower_cookies::Cookies = extract().await?;
    let mut cookie = Cookie::new("token", token);
    cookie.set_max_age(Some(Duration::weeks(4)));
    cookie.set_http_only(true);
    cookie.set_secure(true);
    cookies.add(cookie);
    log!("{user:?}");
    leptos_axum::redirect("/movies");
    Ok(())
}
