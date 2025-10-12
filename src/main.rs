#[cfg(feature = "ssr")]
use color_eyre::Result;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    use std::sync::Arc;

    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use movielister::app::shell;
    use movielister::database::load_from_db;
    use movielister::oauth::oauth::AppState;
    use movielister::{app::App, secrets::init_secrets};

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    init_secrets().await?;

    load_from_db().await?;

    let state = AppState {
        leptos_options: leptos_options,
        states: Arc::default(),
    };

    let app = Router::new()
        // .route("/", get(authenticate))
        // .route("/discord_callback", discord_callback())
        .leptos_routes(&state, routes, {
            let leptos_options = state.leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler::<LeptosOptions, _>(
            shell,
        ))
        .with_state(state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
