#[cfg(feature = "ssr")]
use color_eyre::Result;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    use axum::Router;
    use axum::middleware::{self};
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use movielister::app::shell;
    use movielister::database::{get_auth_tokens, load_from_db};
    use movielister::oauth::oauth::{AppState, authentication_middleware};
    use movielister::{app::App, secrets::init_secrets};
    use std::sync::Arc;
    use tokio::sync::RwLock;

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
        auth_tokens: Arc::new(RwLock::new(get_auth_tokens().await?)),
    };

    let app = Router::new()
        .leptos_routes(&state, routes, {
            let leptos_options = state.leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler::<LeptosOptions, _>(
            shell,
        ))
        .layer(middleware::from_fn(authentication_middleware))
        .layer(tower_cookies::CookieManagerLayer::new())
        .with_state(state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    if cfg!(feature = "fly") {
        log!("listening on http://{}", &addr);
    } else {
        log!("listening on http://localhost:3000");
    }
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
