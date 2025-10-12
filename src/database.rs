use std::sync::OnceLock;

use color_eyre::{
    Result,
    eyre::eyre,
};
use leptos::logging::log;
use serde::{Deserialize, Serialize};
use surrealdb::{
    Surreal,
    engine::any::{self, Any},
    opt::auth::Root,
    sql::Thing,
};

use crate::{app::ssr::MOVIE_LIST, movies::Movie, secrets::get_secrets};

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

const MOVIES: &str = "movie";

#[derive(Serialize, Deserialize)]
struct MovieDBRead {
    id: Thing,
    name: String,
}

impl From<MovieDBRead> for Movie {
    fn from(value: MovieDBRead) -> Self {
        let surrealdb::sql::Id::Number(id) = value.id.id else {
            panic!()
        };
        let id = id as u32;
        Self {
            id,
            name: value.name,
        }
    }
}

pub async fn load_from_db() -> Result<()> {
    init_database().await?;
    let db = get_database().await;
    let movies: Vec<MovieDBRead> = db.select(MOVIES).await?;
    let mut movie_list = MOVIE_LIST
        .write()
        .map_err(|_| eyre!("Failed to open MOVIE_LIST"))?;
    *movie_list = movies.into_iter().map(Movie::from).collect();
    log!("Loaded {} movies", movie_list.len());

    Ok(())
}

pub(crate) async fn write_movie_db(movie: Movie) -> Result<()> {
    let db = get_database().await;
    let _: Option<MovieDBRead> = db.upsert((MOVIES, movie.id as i64)).content(movie).await?;
    Ok(())
}

// const OAUTH_STATE: &str = "oauthstate";

// pub(crate) async fn write_oauth(state: OauthState) -> Result<()> {
//     let db = get_database().await;
//     let _: Option<OauthState> = db
//         .insert((OAUTH_STATE, &state.state))
//         .content(state)
//         .await?;
//     Ok(())
// }

// pub(crate) async fn get_oauth(state: &str) -> Result<OauthState> {
//     let db = get_database().await;
//     let state = db
//         .select((OAUTH_STATE, state))
//         .await?
//         .ok_or_eyre(eyre!("{state} not found"))?;
//     Ok(state)
// }
