use leptos::{
    logging::log,
    prelude::*,
    server::codee::string::FromToStringCodec, server_fn::codec::GetUrl,
};

#[cfg(feature = "ssr")]
pub mod oauth {
    use std::collections::HashMap;
    use std::sync::Arc;

    use axum::extract::FromRef;

    
    use leptos::prelude::ServerFnError;
    use leptos::{config::LeptosOptions, prelude::expect_context};
    use oauth_axum::{
        CustomProvider, OAuthClient,
    };
    use oauth_axum::providers::discord::DiscordProvider;
    use tokio::sync::Mutex;

    use crate::{
        // database::{get_oauth, write_oauth},
        secrets::{Secrets, get_secrets},
    };
    #[derive(Clone)]
    pub struct AppState {
        pub leptos_options: LeptosOptions,
        pub states: Arc<Mutex<HashMap<String, String>>>,
    }

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

    // #[derive(Debug, Default, Deserialize, Serialize, Clone)]
    // pub struct OauthState {
    //     pub(crate) state: String,
    //     pub(crate) verifier: String,
    // }

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

    // pub async fn authenticate(cookies: PrivateCookieJar) -> (PrivateCookieJar, Redirect) {
    //     println!("{cookies:?}");
    //     let cookies = cookies.add(Cookie::new("token", "pizza"));
    //     // let (token, token_write) = token;
    //     // let Some(cookie) = token else {
    //     //     log!("HEHE");
    //     //     leptos_axum::redirect("https://google.com");
    //     //     return Ok(());
    //     // };
    //     // leptos_axum::redirect("/movies");
    //     // Ok(())
    //     (cookies, Redirect::to("https://google.com"))
    // }
}
#[server (prefix="", endpoint="", input = GetUrl)]
pub async fn authenticate() -> Result<(), ServerFnError> {
    use crate::oauth::oauth::create_url;
    use leptos_use::use_cookie;
    let cookies = use_cookie::<String, FromToStringCodec>("token");
    println!("{:?}", cookies.0.get());
    leptos_axum::redirect(&create_url().await?);
    Ok(())
}

#[server (prefix="", endpoint="discord_callback", input = GetUrl)]
pub async fn discord_callback() -> Result<(), ServerFnError> {
    use crate::oauth::oauth::{AppState, QueryAxumCallback};
    use crate::secrets::get_secrets;
    use axum::extract::Query;
    use leptos_axum::extract;
    use oauth::get_client;
    use oauth_axum::OAuthClient;
    use oauth_axum::error::OauthError::{AuthUrlCreationFailed, TokenRequestFailed};
    log!("GOT CALLBACK");
    let state: AppState = expect_context();
    let queries: Query<QueryAxumCallback> = extract().await?;
    let state = state
        .states
        .lock()
        .await
        .remove(&queries.state)
        .ok_or_else(|| ServerFnError::new("Failed to find state"))?;
    let secrets = get_secrets().await;
    match get_client(secrets)
        .generate_token(queries.code.clone(), state)
        .await
    {
        Ok(p) => p,
        Err(e) => match e {
            TokenRequestFailed => panic!("Token request failed"),
            AuthUrlCreationFailed => panic!("auth url creation failed"),
        },
    };
    log!("MADE IT");
    leptos_axum::redirect("/movies");
    Ok(())
}

