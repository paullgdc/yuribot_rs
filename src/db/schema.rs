table! {
    links (id) {
        id -> Integer,
        link -> Text,
        title -> Text,
    }
}

table! {
    links_title_idx (id) {
        id -> Integer,
        link -> Text,
        title -> Text,
        #[sql_name = "links_title_idx"]
        whole_row -> Text,
    }
}
