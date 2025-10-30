
use iddqd::IdHashMap;
use leptos::{
    logging::log, prelude::*, reactive::spawn_local, server_fn::{
        codec::{PostUrl, Rkyv, RkyvEncoding}, BoxedStream, Http, ServerFnError, Websocket
    }
};
use leptos_fetch::{QueryClient, QueryDevtools};
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    StaticSegment,
    components::{Outlet, ParentRoute, Route, Router, Routes},
    hooks::use_navigate,
};
use rkyv::{Archive, Deserialize, Serialize};
use thaw::{ConfigProvider, Theme, ssr::SSRMountStyleProvider};

use crate::{
    app::{movie_list::MovieList, movie_searcher::MovieSearcher},
    movies::Movie,
};

mod movie_searcher;
mod movie_list;

#[derive(Clone, Serialize, Deserialize, Archive, Debug)]
pub(crate) enum Msg {
    AddMovie(Movie),
    RateMovie((u64, u64, u8)),
}

#[cfg(feature = "ssr")]
pub(crate) mod ssr {
    use std::sync::{LazyLock, RwLock};

    use futures::channel::mpsc::Sender;
    use iddqd::IdHashMap;
    use leptos::prelude::ServerFnError;
    use tokio::sync::Mutex;

    use crate::{app::Msg, movies::Movie};
    pub static MOVIE_LIST: LazyLock<RwLock<IdHashMap<Movie>>> = LazyLock::new(RwLock::default);

    pub static SOCKET_LIST: LazyLock<Mutex<Vec<Sender<Result<Msg, ServerFnError>>>>> =
        LazyLock::new(Mutex::default);
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <SSRMountStyleProvider>
            <!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta charset="utf-8" />
                    <meta name="viewport" content="width=device-width, initial-scale=1" />
                    <AutoReload options=options.clone() />
                    <HydrationScripts options />
                    <MetaTags />
                </head>
                <body>
                    <App />
                </body>
            </html>
        </SSRMountStyleProvider>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    let client = QueryClient::new()
        .with_refetch_enabled_toggle(true)
        .provide();

    let theme = RwSignal::new(Theme::dark());

    view! {
        <QueryDevtools client=client />
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/movielister.css" />

        // sets the document title
        <Title text="Movie Lister" />

        // content for this welcome page
        <ConfigProvider theme>
            <Router>
                <main>
                    <Routes fallback=|| "Page not found.".into_view()>
                        <ParentRoute path=StaticSegment("/movies") view=HomePage>
                            <Route path=StaticSegment("/list") view=MovieList />
                            <Route path=StaticSegment("/new") view=MovieSearcher />
                        </ParentRoute>
                    </Routes>
                </main>
            </Router>
        </ConfigProvider>
    }
}

#[server]
async fn get_movies() -> Result<IdHashMap<Movie>, ServerFnError> {
    let count = self::ssr::MOVIE_LIST.read()?;
    Ok(count.clone())
}

#[server]
async fn get_movie_list() -> Result<Vec<u64>, ServerFnError> {
    let movies = self::ssr::MOVIE_LIST.read()?;
    Ok(movies.iter().map(|m| m.base.movie_id).collect())
}

#[server]
pub(crate) async fn get_movie(id: u64) -> Result<Movie, ServerFnError> {
    let movies = self::ssr::MOVIE_LIST.read()?;
    movies.get(&id).cloned()
        .ok_or_else(|| ServerFnError::ServerError(format!("Couldn't find movie {id}")))
}

#[server(protocol = Http<PostUrl, Rkyv>)]
pub(crate) async fn add_movie(movie_id: u64) -> Result<(), ServerFnError> {
    use crate::secrets::get_secrets;
    use tmdb_api::client::Client;
    use tmdb_api::client::reqwest::ReqwestExecutor;
    use chrono::Utc;
    let client = Client::<ReqwestExecutor>::new(get_secrets().await.tmdb_api_key.clone());
    let list = client
        .get_movie_details(movie_id, &Default::default())
        .await?;
    let movie: Movie = Movie {
        base: list.inner.into(),
        time_added: Utc::now().timestamp() as u64,
        ..Default::default()
    };
    use futures::SinkExt;
    let movie_send = movie.clone();
    tokio::spawn(async move {
        let mut list = self::ssr::SOCKET_LIST.lock().await;
        let list: &mut Vec<_> = list.as_mut();
        list.retain(|s| !s.is_closed());
        for socket in list {
            socket
                .send(Ok(Msg::AddMovie(movie_send.clone())))
                .await
                .unwrap();
        }
    });
    use crate::database::write_movie_db;
    self::ssr::MOVIE_LIST
        .write()?
        .insert_overwrite(movie.clone());
    write_movie_db(movie).await.map_err(ServerFnError::new)?;
    Ok(())
}

#[server(protocol = Websocket<RkyvEncoding, RkyvEncoding>)]
async fn get_socket(
    _input: BoxedStream<Msg, ServerFnError>,
) -> Result<BoxedStream<Msg, ServerFnError>, ServerFnError> {
    let (tx, rc) = futures::channel::mpsc::channel(10);
    self::ssr::SOCKET_LIST.lock().await.push(tx);
    Ok(rc.into())
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let client: QueryClient = expect_context();

    use futures::{StreamExt, channel::mpsc};
    let (_tx, rx) = mpsc::channel(1);
    if cfg!(feature = "hydrate") {
        spawn_local(async move {
            match get_socket(rx.into()).await {
                Ok(mut messages) => {
                    while let Some(msg) = messages.next().await {
                        let Ok(msg) = msg else {
                            continue;
                        };
                        match msg {
                            Msg::AddMovie(movie) => {
                                client.update_query(get_movies, (), |c| {
                                    if let Some(Ok(c)) = c {
                                        c.insert_overwrite(movie);
                                    }
                                });
                            }
                            Msg::RateMovie((movie_id, user_id, rating)) => {
                                client.update_query(get_movies, (), |c| {
                                    client.untrack_update_query();
                                    if let Some(Ok(c)) = c {
                                        if let Some(mut movie) = c.get_mut(&movie_id) {
                                            movie.rating.add_rating(user_id, rating);
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
                Err(e) => leptos::logging::warn!("{e}"),
            }
        });
    }

    view! {
        <h1>"Welcome to MovieLister!"</h1>
        <button on:click=move |_| use_navigate()(
            "/movies/list",
            leptos_router::NavigateOptions::default(),
        )>"List"</button>
        <button on:click=move |_| use_navigate()(
            "/movies/new",
            leptos_router::NavigateOptions::default(),
        )>"Search"</button>
        <Outlet />
    }
}

