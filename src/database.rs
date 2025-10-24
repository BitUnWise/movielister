use std::sync::OnceLock;

use color_eyre::{Result, eyre::eyre};
use iddqd::IdHashMap;
use leptos::logging::log;
use surrealdb::{
    Surreal,
    engine::any::{self, Any},
    opt::auth::Root,
};

use crate::{
    app::ssr::MOVIE_LIST,
    movies::Movie,
    oauth::oauth::{AUTH_TOKENS, User},
    secrets::get_secrets,
};

static DB_CONNECTION: OnceLock<Surreal<Any>> = OnceLock::new();

async fn init_database() -> Result<()> {
    let secrets = get_secrets().await;
    // Open a connection
    let surreal_db =
        any::connect("wss://mrspeoplebot-06a4qd2dq5plh88gvemepf89to.aws-use1.surreal.cloud")
            .await?;

    // Select a namespace and database
    surreal_db
        .use_ns("movielister")
        .use_db("movielister")
        .await?;

    // Authenticate
    surreal_db
        .signin(Root {
            username: "MrsPeopleBot",
            password: &secrets.surreal_db_password,
        })
        .await?;
    log!("DataBase init");
    DB_CONNECTION
        .set(surreal_db)
        .map_err(|_| eyre!("Failed to set database"))
}

async fn get_database() -> &'static Surreal<Any> {
    DB_CONNECTION.get().unwrap()
}

pub(crate) const MOVIES: &str = "movie";

pub async fn load_from_db() -> Result<()> {
    init_database().await?;
    let db = get_database().await;
    let movies: Vec<Movie> = db.select(MOVIES).await?;
    let mut movie_list = MOVIE_LIST
        .write()
        .map_err(|_| eyre!("Failed to open MOVIE_LIST"))?;
    *movie_list = movies.into_iter().collect();
    log!("Loaded {} movies", movie_list.len());

    Ok(())
}

pub(crate) async fn write_movie_db(movie: Movie) -> Result<()> {
    let db = get_database().await;
    let _: Option<Movie> = db
        .upsert((MOVIES, movie.base.movie_id as i64))
        .content(movie)
        .await?;
    Ok(())
}

pub(crate) const USERS: &str = "users";

pub async fn get_users() -> Result<IdHashMap<User>> {
    let db = get_database().await;
    let users: Vec<User> = db.select(USERS).await?;
    let mut auth_tokens = AUTH_TOKENS.write().await;
    for user in &users {
        for token in &user.auth_tokens {
            auth_tokens.insert(token.to_owned(), user.user_id);
        }
    }
    log!("Loaded {} users", users.len());
    let users = users.into_iter().collect();
    Ok(users)
}

pub(crate) async fn write_auth_token(user: u64, token: &str) -> Result<()> {
    let db = get_database().await;
    let query = format!("UPDATE {USERS}:⟨{user}⟩ SET auth_tokens += \"{token}\"");
    log!("adding {query}");
    db.query(query).await?;
    log!("DID IT!");
    Ok(())
}
