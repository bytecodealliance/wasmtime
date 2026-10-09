bindgen!({
    inline: r#"
        package example:exported-resources;

        world export-some-resources {
            export logging;
        }

        interface logging {
            enum level {
                debug,
                info,
                warn,
                error,
            }

            resource logger {
                constructor(max-level: level);

                get-max-level: func() -> level;
                set-max-level: func(level: level);

                log: func(level: level, msg: string);
            }

            inspect-logger: func(logger: borrow<logger>) -> level;
            take-logger: func(logger: logger);
            get-default-logger: func() -> logger;
        }
    "#,
});
