bindgen!({
    inline: r#"
        package example:store-in-imports;

        world my-world {
            import sync-with-store: func();
            import async-with-store: async func();

            import sync-without-store: func();
            import async-without-store: func();

            export run: async func();
        }
    "#,

    imports: {
        "sync-with-store": store,
        // note that this isn't required because WIT-level `async` functions
        // always have access to the store.
        // "async-with-store": store,
        "async-without-store": async,
    },
});
