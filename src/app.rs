use std::time::Duration;

use leptos::{prelude::*, task::spawn_local};
use leptos_fetch::{QueryClient, QueryDevtools, QueryOptions, QueryScope};
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    StaticSegment,
    components::{Route, Router, Routes},
};

#[cfg(feature = "ssr")]
pub mod ssr {
    use std::sync::{LazyLock, RwLock};
    pub static COUNT: LazyLock<RwLock<u32>> = LazyLock::new(RwLock::default);
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
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
        <QueryDevtools client=client/>
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/movielister.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

#[server]
async fn get_count() -> Result<u32, ServerFnError> {
    let count = self::ssr::COUNT.read()?;
    Ok(*count)
}

#[server]
async fn inc_count() -> Result<(), ServerFnError> {
    *self::ssr::COUNT.write()? += 1;
    Ok(())
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let client: QueryClient = expect_context();

    let query = QueryScope::new(get_count)
        .with_options(QueryOptions::new().with_refetch_interval(Duration::from_secs(60)))
        .with_title("Count");
    let resource = client.resource(query, move || ());

    let update_count = ServerAction::<IncCount>::new();

    // Effect::new(move |_| {
    //     update_count.version().get();
    //     client.invalidate_query_scope(get_count);
    // });

    let inc_click = move |_| {
        spawn_local(async move {
            update_count.dispatch(IncCount {});
            client.update_query(get_count, (), |c| {
                if let Some(Ok(c)) = c {
                    *c += 1
                }
            });
        });
    };

    view! {
        <h1>"Welcome to Leptos!"</h1>
        <Suspense fallback=move || view! {<p>"Loading list"</p>}>
            {move || Suspend::new(async move {
                let resource = resource.await;
                view!{
                <button on:click=inc_click
                >"Click Me: " {resource}</button>
                }
            })}
        </Suspense>
    }
}
