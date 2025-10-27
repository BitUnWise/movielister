use leptos::prelude::*;
use thaw::{Button, ButtonAppearance};

use crate::app::movie_list::{SortOrder, SortType};

#[component]
pub(crate) fn sort_button(sort_type: SortType, sort_order: RwSignal<SortOrder>) -> impl IntoView {
    let appearance = Signal::derive(move || get_style(sort_type, sort_order.get()));
    let icon = Signal::derive(move || {
        (sort_type == sort_order.get().sort_type).then(|| {
            if sort_order.get().reversed {
                icondata::AiDownOutlined
            } else {
                icondata::AiUpOutlined
            }
        })
    });
    let on_click = move |_| {
        if sort_order.get().sort_type == sort_type {
            sort_order.update(|s| s.reversed = !s.reversed);
        } else {
            sort_order.update(|s| {
                s.sort_type = sort_type;
                s.reversed = false
            });
        }
    };
    view! {
        <Button appearance icon on_click>
            {format!("{sort_type}")}
        </Button>
    }
}

fn get_style(sort_type: SortType, sort_order: SortOrder) -> ButtonAppearance {
    if sort_type == sort_order.sort_type {
        ButtonAppearance::Primary
    } else {
        ButtonAppearance::Secondary
    }
}
