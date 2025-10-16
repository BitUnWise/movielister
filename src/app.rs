use std::time::Duration;

use iddqd::IdHashMap;
use leptos::{
    prelude::*,
    reactive::spawn_local,
    server_fn::{
        BoxedStream, Http, ServerFnError, Websocket,
        codec::{PostUrl, Rkyv, RkyvEncoding},
    },
};
use leptos_fetch::{QueryClient, QueryDevtools, QueryOptions, QueryScope};
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    StaticSegment,
    components::{Route, Router, Routes},
};
use rkyv::{Archive, Deserialize, Serialize};

use crate::{app::movie_searcher::MovieSearcher, movies::Movie};

mod movie_searcher;

#[derive(Clone, Serialize, Deserialize, Archive, Debug)]
pub(crate) enum Msg {
    AddMovie(Movie),
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
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    let client = QueryClient::new()
        .with_refetch_enabled_toggle(true)
        .provide();

    view! {
        <QueryDevtools client=client />
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/movielister.css" />

        // sets the document title
        <Title text="Movie Lister" />

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("/movies") view=HomePage />
                </Routes>
            </main>
        </Router>
    }
}

#[server]
async fn get_movies() -> Result<IdHashMap<Movie>, ServerFnError> {
    let count = self::ssr::MOVIE_LIST.read()?;
    Ok(count.clone())
}

#[server(protocol = Http<PostUrl, Rkyv>)]
async fn add_movie(movie: Movie) -> Result<(), ServerFnError> {
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
                        }
                    }
                }
                Err(e) => leptos::logging::warn!("{e}"),
            }
        });
    }

    let query = QueryScope::new(get_movies)
        .with_options(QueryOptions::new().with_refetch_interval(Duration::from_secs(360)))
        .with_title("Movies");
    let resource = client.resource(query, move || ());

    view! {
        <h1>"Welcome to MovieLister!"</h1>
        <MovieSearcher />
        <Suspense fallback=move || {
            view! { <p>"Loading list"</p> }
        }>
        <h1>"Welcome to MovieLister!"</h1>
            {move || Suspend::new(async move {
                let resource = resource.await.expect("Should have movies");
                resource
                    .iter()
                    .map(
                        &move |movie: &Movie| {
                            let name = movie.name.clone();
                            view! { <p>"Title: " {name}</p> }
                        },
                    )
                    .collect::<Vec<_>>()
            })}
        </Suspense>
    }
}
