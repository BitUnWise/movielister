use std::{collections::HashMap, sync::Arc};

use axum::{extract::Query, response::Redirect, Extension};
use leptos::logging::log;
use oauth_axum::{error::OauthError::{AuthUrlCreationFailed, TokenRequestFailed}, providers::discord::DiscordProvider, CustomProvider, OAuthClient};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    database::{get_oauth, write_oauth},
    secrets::{get_secrets, Secrets},
};

const REDIRECT: &str = "https://discord.com/oauth2/authorize?client_id=1426088667113586858&response_type=code&redirect_uri=http%3A%2F%2F127.0.0.1%3A3000%2F&scope=identify";

fn get_client(secrets: &Secrets) -> CustomProvider {
    DiscordProvider::new(
        secrets.discord_client_id.clone(),
        secrets.discord_client_secret.clone(),
        "http://localhost:3000/discord_callback".to_owned(),
    )
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct OauthState {
    pub(crate) state: String,
    pub(crate) verifier: String,
}

// pub static ID_LIST: LazyLock<RwLock<Vec<OauthState>>> = LazyLock::new(RwLock::default);

#[axum::debug_handler]
pub async fn create_url() -> Redirect {
    let secrets = get_secrets().await;
    let state_oauth = get_client(secrets)
        .generate_url(vec!["identify".to_owned(), "openid".to_owned()], |state_e| async move {
            //SAVE THE DATA IN THE DB OR MEMORY
            //state should be your ID
            // state.lock().await.push(OauthState {
            //     id: state_e.state,
            //     verifier: state_e.verifier,
            // });
            // state.states.insert(state_e.state, state_e.verifier);
            write_oauth(OauthState { state: state_e.state, verifier: state_e.verifier }).await.unwrap();
        })
        .await
        .ok()
        .unwrap()
        .state
        .unwrap();

    Redirect::to(&state_oauth.url_generated.unwrap())
}

#[derive(Clone, serde::Deserialize)]
pub struct QueryAxumCallback {
    pub code: String,
    pub state: String,
}

pub async fn callback(Query(queries): Query<QueryAxumCallback>) -> String {
    // GET DATA FROM DB OR MEMORY
    // get data using state as ID
    let state = get_oauth(&queries.state).await.unwrap();
    let secrets = get_secrets().await;
    match get_client(secrets)
        .generate_token(queries.code, state.verifier.to_owned())
        .await {
            Ok(p) => p,
            Err(e) => match e {
                TokenRequestFailed => panic!("Token request failed"),
                AuthUrlCreationFailed => panic!("auth url creation failed"),
            }
        }
        
}
