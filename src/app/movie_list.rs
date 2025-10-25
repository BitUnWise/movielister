use std::time::Duration;

use iddqd::IdHashMap;
use leptos::prelude::*;
use leptos_fetch::{QueryClient, QueryOptions, QueryScope};
use thaw::{Tab, TabList};

use crate::{
    app::get_movies,
    movies::{Movie, MovieCard},
};

#[component]
pub(crate) fn movie_list() -> impl IntoView {
    let client: QueryClient = expect_context();
    let query = QueryScope::new(get_movies)
        .with_options(QueryOptions::new().with_refetch_interval(Duration::from_secs(360)))
        .with_title("Movies");
    let resource = client.resource(query, move || ());
    let sort_key = RwSignal::new("title".to_string());
    let sort_order = client.local_resource(
        async | sort: String | { SortOrder::from_str(sort.as_str()) },
        move || sort_key.get(),
    );
    view! {
        <Suspense fallback=move || {
            view! { <p>"Loading list"</p> }
        }>
            <TabList selected_value=sort_key>
                <Tab value="title">Title</Tab>
                <Tab value="score">Score</Tab>
            </TabList>
            <div class="search">
                {move || Suspend::new(async move {
                    let resource = resource.await.expect("Should have movies");
                    let sort_order = sort_order.await;
                    let sorted = sort_movies(sort_order, &resource);
                    sorted
                        .iter()
                        .map(|t| resource.get(t).unwrap())
                        .map(
                            &move |movie: &Movie| {
                                view! { <MovieCard movie=movie.clone() /> }
                            },
                        )
                        .collect::<Vec<_>>()
                })}
            </div>
        </Suspense>
    }
}

fn sort_movies(sort_order: SortOrder, movies: &IdHashMap<Movie>) -> Vec<u64> {
    let mut titles = movies.iter().map(|m| m.base.movie_id).collect::<Vec<_>>();
    match sort_order {
        SortOrder::Title => titles.sort_unstable_by_key(|t| &movies.get(t).unwrap().base.title),
        SortOrder::Score => titles
            .sort_unstable_by_key(|t| (movies.get(t).unwrap().base.vote_average * 100.) as u32),
    }
    titles
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SortOrder {
    Title,
    Score,
}

impl SortOrder {
    fn from_str(s: &str) -> Self {
        match s {
            "title" => Self::Title,
            "score" => Self::Score,
            _ => panic!("unknown sort key"),
        }
    }
}
