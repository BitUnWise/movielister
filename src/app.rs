use std::time::Duration;

use iddqd::IdHashMap;
use leptos::prelude::*;
use leptos_fetch::{QueryClient, QueryDevtools, QueryOptions, QueryScope};
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    StaticSegment,
    components::{Route, Router, Routes},
};

use crate::movies::Movie;

#[cfg(feature = "ssr")]
pub(crate) mod ssr {
    use std::sync::{LazyLock, RwLock};

    use iddqd::IdHashMap;

    use crate::movies::Movie;
    pub static MOVIE_LIST: LazyLock<RwLock<IdHashMap<Movie>>> = LazyLock::new(RwLock::default);
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
        <Title text="Welcome to Leptos" />

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage />
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

#[server]
async fn add_movie(movie: Movie) -> Result<(), ServerFnError> {
    use crate::database::write_movie_db;
    self::ssr::MOVIE_LIST
        .write()?
        .insert_overwrite(movie.clone());
    write_movie_db(movie).await.map_err(ServerFnError::new)?;
    Ok(())
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let client: QueryClient = expect_context();

    let query = QueryScope::new(get_movies)
        .with_options(QueryOptions::new().with_refetch_interval(Duration::from_secs(60)))
        .with_title("Movies");
    let resource = client.resource(query, move || ());

    let add_movie_action = ServerAction::<AddMovie>::new();

    Effect::new(move |_| {
        if add_movie_action.pending().get() {
            let movie = add_movie_action.input().get().unwrap().movie;

            client.update_query(get_movies, (), |c| {
                if let Some(Ok(c)) = c {
                    c.insert_overwrite(movie);
                }
            });
        }
    });

    view! {
        <h1>"Welcome to Leptos!"</h1>
        <Suspense fallback=move || view! { <p>"Loading list"</p> }>
            <ActionForm action=add_movie_action>
                <input type="text" name="movie[name]" />
                <input type="number" name="movie[id]" />
                <input type="submit" />
            </ActionForm>
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
