use std::collections::{BTreeMap, HashMap};

use leptos::{logging::log, prelude::*, reactive::spawn_local};
use rkyv::{Archive, Deserialize as RDes, Serialize as RSer};
use serde::{Deserialize, Serialize};
use thaw::{
    Button, ButtonAppearance, Dialog, DialogActions, DialogContent, DialogSurface, DialogTitle, Label, Rating as TRating, RatingDisplay
};




#[server]
async fn rate_movie(movie_id: u64, rating: u8) -> Result<(), ServerFnError> {
    use crate::app::ssr::MOVIE_LIST;
    use crate::app::{ssr::SOCKET_LIST, Msg};
    use crate::oauth::oauth::get_user;
    use futures::SinkExt;
    let user = get_user().await?;
    if let Some(mut movie) = MOVIE_LIST.write()?.get_mut(&movie_id) {
        movie.rating.add_rating(user, rating);
    }
    tokio::spawn(async move {
        let mut list = SOCKET_LIST.lock().await;
        let list: &mut Vec<_> = list.as_mut();
        list.retain(|s| !s.is_closed());
        for socket in list {
            socket
                .send(Ok(Msg::RateMovie((movie_id, user, rating))))
                .await
                .unwrap();
        }
    });
    Ok(())
}

#[component]
pub(crate) fn Rating(rating: MovieRating, id: u64) -> impl IntoView {
    let is_rating = RwSignal::new(false);
    let current_rating = RwSignal::new(0.0);
    let rating_disabled = Signal::derive(move || current_rating.get() == 0.0);
    let rating_value = rating.average as f32 / 10.0;
    log!("Made rating");
    view! {
        <RatingDisplay value=rating_value max=10 on:click=move |_| is_rating.set(true) />
        <Dialog open=is_rating>
            <DialogSurface>
                <DialogTitle>"Rating"</DialogTitle>
                <DialogContent>
                    <Label>{move || current_rating.get()}</Label>
                    <TRating value=current_rating max=10 step=0.5 />
                </DialogContent>
                <DialogActions>
                    <Button
                        on:click=move |_| {
                            is_rating.set(false);
                            let rating = current_rating.get();
                            spawn_local(async move {
                                let _ = rate_movie(id, (rating * 10.0) as u8).await;
                            })
                        }
                        disabled=rating_disabled
                    >
                        "Rate"
                    </Button>
                    <Button
                        on:click=move |_| {
                            current_rating.set(0.0);
                            is_rating.set(false);
                        }
                        appearance=ButtonAppearance::Secondary
                    >
                        "Cancel"
                    </Button>
                </DialogActions>

            </DialogSurface>
        </Dialog>
    }
}

#[derive(Default, Eq, PartialEq, Serialize, Deserialize, Archive, Debug, RDes, RSer, Clone)]
pub(crate) struct MovieRating {
    average: u8,
    ratings: HashMap<u64, u8>,
}

impl MovieRating {
    pub(crate) fn add_rating(&mut self, user: u64, rating: u8) {
        self.ratings.insert(user, rating);
        self.update_rating();
    }

    pub(crate) fn update_rating(&mut self) {
        let sum: usize = self.ratings.values().map(|r| *r as usize).sum();
        self.average = (sum / self.ratings.len()) as u8;
    }
}

// impl PartialOrd for MovieRating {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         Some(self.cmp(other))
//     }
// }

// impl Ord for MovieRating {
//     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//         self.average.cmp(&other.average)
//     }
// }

