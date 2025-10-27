use std::{cmp::Ordering, time::Duration};

use iddqd::IdHashMap;
use leptos::prelude::*;
use leptos_fetch::{QueryClient, QueryOptions, QueryScope};
use thaw::Flex;

use crate::{
    app::{get_movies, movie_list::sort_button::SortButton},
    movies::{Movie, MovieCard},
};

mod sort_button;

#[component]
pub(crate) fn movie_list() -> impl IntoView {
    let client: QueryClient = expect_context();
    let query = QueryScope::new(get_movies)
        .with_options(QueryOptions::new().with_refetch_interval(Duration::from_secs(360)))
        .with_title("Movies");
    let resource = client.resource(query, move || ());
    let sort_order = RwSignal::new(SortOrder::default());
    view! {
        <Flex>
            <SortButton sort_type=SortType::Title sort_order />
            <SortButton sort_type=SortType::Score sort_order />
        </Flex>
        <Suspense fallback=move || {
            view! { <p>"Loading list"</p> }
        }>
            <div class="search">
                {move || Suspend::new(async move {
                    let resource = resource.await.expect("Should have movies");
                    let ids = sort_order.get().sort_movies(&resource);
                    ids.iter()
                        .map(|i| resource.get(i).unwrap())
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default, Hash)]
struct SortOrder {
    reversed: bool,
    sort_type: SortType,
}

impl SortOrder {
    fn sort_movies(&self, movies: &IdHashMap<Movie>) -> Vec<u64> {
        let mut titles = movies.iter().map(|m| m.base.movie_id).collect::<Vec<_>>();
        match self.sort_type {
            SortType::Title => {
                let key = |t| &movies.get(&t).unwrap().base.title;
                titles.sort_unstable_by(|l, r| self.sort_items(*l, *r, key));
            }
            SortType::Score => {
                let key = |t| (movies.get(&t).unwrap().base.vote_average * 100.) as u32;
                titles.sort_unstable_by(|l, r| self.sort_items(*l, *r, key));
            }
        }
        titles
    }

    fn sort_items<I: Ord>(&self, l: u64, r: u64, key: impl Fn(u64) -> I) -> Ordering {
        let l = key(l);
        let r = key(r);
        if self.reversed {
            r.cmp(&l)
        } else {
            l.cmp(&r)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default, Hash)]
enum SortType {
    #[default]
    Title,
    Score,
}

use std::fmt::Display as StdDisplay;

impl StdDisplay for SortType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortType::Title => write!(f, "Title"),
            SortType::Score => write!(f, "Score"),
        }
    }
}
