// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! Feather-style SVG icon functions. Each returns an Element for embedding inside IBtn/ILink.

use dioxus::prelude::*;

fn svg(content: Element) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            width: "16",
            height: "16",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            {content}
        }
    }
}

pub fn icon_home() -> Element {
    svg(rsx! {
        path { d: "M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" }
        polyline { points: "9 22 9 12 15 12 15 22" }
    })
}

pub fn icon_settings() -> Element {
    svg(rsx! {
        circle { cx: "12", cy: "12", r: "3" }
        path { d: "M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" }
    })
}

pub fn icon_logout() -> Element {
    svg(rsx! {
        path { d: "M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" }
        polyline { points: "16 17 21 12 16 7" }
        line { x1: "21", y1: "12", x2: "9", y2: "12" }
    })
}

pub fn icon_plus() -> Element {
    svg(rsx! {
        line { x1: "12", y1: "5", x2: "12", y2: "19" }
        line { x1: "5", y1: "12", x2: "19", y2: "12" }
    })
}

pub fn icon_check() -> Element {
    svg(rsx! {
        polyline { points: "20 6 9 17 4 12" }
    })
}

pub fn icon_x() -> Element {
    svg(rsx! {
        line { x1: "18", y1: "6", x2: "6", y2: "18" }
        line { x1: "6", y1: "6", x2: "18", y2: "18" }
    })
}

pub fn icon_trash() -> Element {
    svg(rsx! {
        polyline { points: "3 6 5 6 21 6" }
        path { d: "M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" }
        path { d: "M10 11v6M14 11v6M9 6V4h6v2" }
    })
}

pub fn icon_play() -> Element {
    svg(rsx! {
        polygon { points: "5 3 19 12 5 21 5 3" }
    })
}

pub fn icon_folder() -> Element {
    svg(rsx! {
        path { d: "M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" }
    })
}

pub fn icon_arrow_up() -> Element {
    svg(rsx! {
        line { x1: "12", y1: "19", x2: "12", y2: "5" }
        polyline { points: "5 12 12 5 19 12" }
    })
}

pub fn icon_arrow_left() -> Element {
    svg(rsx! {
        line { x1: "19", y1: "12", x2: "5", y2: "12" }
        polyline { points: "12 19 5 12 12 5" }
    })
}

pub fn icon_arrow_right() -> Element {
    svg(rsx! {
        line { x1: "5", y1: "12", x2: "19", y2: "12" }
        polyline { points: "12 5 19 12 12 19" }
    })
}

pub fn icon_chevron_left() -> Element {
    svg(rsx! {
        polyline { points: "15 18 9 12 15 6" }
    })
}

pub fn icon_chevron_right() -> Element {
    svg(rsx! {
        polyline { points: "9 18 15 12 9 6" }
    })
}

pub fn icon_clock() -> Element {
    svg(rsx! {
        circle { cx: "12", cy: "12", r: "10" }
        polyline { points: "12 6 12 12 16 14" }
    })
}

pub fn icon_terminal() -> Element {
    svg(rsx! {
        polyline { points: "4 17 10 11 4 5" }
        line { x1: "12", y1: "19", x2: "20", y2: "19" }
    })
}

pub fn icon_document() -> Element {
    svg(rsx! {
        path { d: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" }
        polyline { points: "14 2 14 8 20 8" }
    })
}

pub fn icon_key() -> Element {
    svg(rsx! {
        circle { cx: "7.5", cy: "15.5", r: "5.5" }
        polyline { points: "21 2 13 10" }
        line { x1: "21", y1: "7", x2: "16", y2: "2" }
    })
}

pub fn icon_user_plus() -> Element {
    svg(rsx! {
        path { d: "M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" }
        circle { cx: "12", cy: "7", r: "4" }
        line { x1: "19", y1: "8", x2: "19", y2: "14" }
        line { x1: "22", y1: "11", x2: "16", y2: "11" }
    })
}

pub fn icon_lock() -> Element {
    svg(rsx! {
        rect { x: "3", y: "11", width: "18", height: "11", rx: "2", ry: "2" }
        path { d: "M7 11V7a5 5 0 0 1 10 0v4" }
    })
}

pub fn icon_refresh() -> Element {
    svg(rsx! {
        polyline { points: "23 4 23 10 17 10" }
        polyline { points: "1 20 1 14 7 14" }
        path { d: "M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" }
    })
}

pub fn icon_calendar() -> Element {
    svg(rsx! {
        rect { x: "3", y: "4", width: "18", height: "18", rx: "2", ry: "2" }
        line { x1: "16", y1: "2", x2: "16", y2: "6" }
        line { x1: "8", y1: "2", x2: "8", y2: "6" }
        line { x1: "3", y1: "10", x2: "21", y2: "10" }
    })
}
