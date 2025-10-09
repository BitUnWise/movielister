use chrono::NaiveDate;
use iddqd::{IdHashItem, id_upcast};
use leptos::prelude::*;
use rkyv::{Archive, Deserialize as RDes, Serialize as RSer};
use serde::{Deserialize, Serialize};

#[derive(
    Default,
    Clone,
    PartialEq,
    PartialOrd,
    Ord,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    Debug,
    RDes,
    RSer,
)]
pub(crate) struct Movie {
    pub(crate) id: u32,
    pub(crate) name: String,
}

impl IdHashItem for Movie {
    type Key<'a> = u32;

    fn key(&self) -> Self::Key<'_> {
        self.id
    }

    id_upcast!();
}

#[derive(
    Default, Clone, PartialEq, PartialOrd, Serialize, Deserialize, Archive, Debug, RDes, RSer,
)]
pub(crate) struct ReleaseDate(i32);

impl From<NaiveDate> for ReleaseDate {
    fn from(value: NaiveDate) -> Self {
        Self(value.to_epoch_days())
    }
}

#[derive(
    Default, Clone, PartialEq, PartialOrd, Serialize, Deserialize, Archive, Debug, RDes, RSer,
)]
pub(crate) struct MovieBase {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) original_title: String,
    pub(crate) original_language: String,
    pub(crate) overview: String,
    pub(crate) release_date: Option<ReleaseDate>,
    pub(crate) poster_path: Option<String>,
    pub(crate) backdrop_path: Option<String>,
    pub(crate) adult: bool,
    pub(crate) popularity: f64,
    pub(crate) vote_count: u64,
    pub(crate) vote_average: f64,
    pub(crate) video: bool,
}

#[cfg(feature = "ssr")]
impl From<tmdb_api::movie::MovieBase> for MovieBase {
    fn from(value: tmdb_api::movie::MovieBase) -> Self {
        Self {
            id: value.id,
            title: value.title,
            original_title: value.original_title,
            original_language: value.original_language,
            overview: value.overview,
            release_date: value.release_date.map(ReleaseDate::from),
            poster_path: value.poster_path,
            backdrop_path: value.backdrop_path,
            adult: value.adult,
            popularity: value.popularity,
            vote_count: value.vote_count,
            vote_average: value.vote_average,
            video: value.video,
        }
    }
}

#[derive(
    Default, Clone, PartialEq, PartialOrd, Serialize, Deserialize, Archive, Debug, RDes, RSer,
)]
pub struct MovieSearch {
    pub(crate) inner: MovieBase,
    pub(crate) genre_ids: Vec<u64>,
}

#[cfg(feature = "ssr")]
impl From<tmdb_api::movie::MovieShort> for MovieSearch {
    fn from(value: tmdb_api::movie::MovieShort) -> Self {
        Self {
            inner: value.inner.into(),
            genre_ids: value.genre_ids,
        }
    }
}

const IMAGE_PREFIX: &str = "https://image.tmdb.org/t/p/w500/";

#[component]
pub(crate) fn MovieThumb(movie: MovieSearch) -> impl IntoView {
    let poster = movie
        .inner
        .poster_path
        .map(|p| format!("{IMAGE_PREFIX}{p}"))
        .unwrap_or_default();
    view! {
        <div class = "card">
            <img src=poster />
            <label>{movie.inner.title}</label>
        </div>
    }
}
