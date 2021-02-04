table! {
    links (id) {
        id -> Integer,
        title -> Text,
        target -> Text,
        code -> Text,
        author -> Integer,
        created_at -> Timestamp,
    }
}

table! {
    users (id) {
        id -> Integer,
        username -> Text,
        email -> Text,
        password -> Text,
    }
}

joinable!(links -> users (author));

allow_tables_to_appear_in_same_query!(links, users,);
