fn main() {
    let vars = std::env::vars().collect::<std::collections::HashMap<_, _>>();
    assert_eq!(
        vars,
        [("frabjous", "day"), ("callooh", "callay")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect()
    );
}
