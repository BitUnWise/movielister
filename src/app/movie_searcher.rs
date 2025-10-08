use leptos::{
    prelude::*,
    server_fn::{
         codec::{PostUrl, Rkyv}, Http
    },
};
use leptos_fetch::QueryClient;

use crate::movies::MovieSearch;

#[server(protocol = Http<PostUrl, Rkyv>)]
async fn get_search(query: String) -> Result<Vec<MovieSearch>, ServerFnError> {
    if query.is_empty() {
        return Ok(vec![]);
    }
    use crate::secrets::get_secrets;
use tmdb_api::client::reqwest::ReqwestExecutor;
    use tmdb_api::{client::Client,  movie::search::Params};
    let client = Client::<ReqwestExecutor>::new(get_secrets().await.tmdb_api_key.clone());
    let list = client.search_movies(query, &Params::default()).await?;
    Ok(list.results.into_iter().map(MovieSearch::from).collect())
}

#[component]
pub fn movie_searcher() -> impl IntoView {
    let client: QueryClient = expect_context();

    let search_text = RwSignal::new("".to_string());
    let current_text = RwSignal::new("".to_string());

    let search = client.resource(get_search, move || search_text.get());

    view! {
        <input type="text" bind:value=current_text />

        <button on:click=move |_| { search_text.set(current_text.get()) }>"Search"</button>

        <Suspense fallback=move || {
            view! {}
        }>
            {move || Suspend::new(async move {
                let search = search.await.expect("Should have movies");
                search
                    .iter()
                    .map(
                        &move |movie: &MovieSearch| {
                            let name = movie.inner.title.clone();
                            view! { <p>"Title: " {name}</p> }
                        },
                    )
                    .collect::<Vec<_>>()
            })}
        </Suspense>
    }
}
