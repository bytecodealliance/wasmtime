bindgen!({
    inline: r#"
package example:resource-async;

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
        /// May block if the host's log buffer is full (i.e. backpressure).
        log: async func(level: level, msg: string);
    }
}

world app {
    import logging;
    /// Because `logging.logger.log` is async, `run` must also be async; otherwise,
    /// a trap will occur at runtime if `run` calls `log` and it blocks.
    export run: async func();
}
"#,

    imports: {

        // This is the notation that's used to flag individual methods:
        //
        // "example:resource-async/logging.[constructor]logger": trappable,
        // "example:resource-async/logging.[method]logger.get-max-level": trappable,
        // "example:resource-async/logging.[method]logger.set-max-level": trappable,
        // "example:resource-async/logging.[method]logger.log": trappable,
        //
        // Note that if one resource method specifies a flag, the remaining resource methods
        // are no longer covered by e.g. `default: trappable`, and must be flagged
        // individually.
        default: trappable,
    },

    with: {
         // Specify that our host resource is going to point to the `MyLogger`
        // which is defined just below this macro.
        "example:resource-async/logging.logger": MyLogger,
    },
});

/// A sample host-defined type which contains arbitrary host-defined data.
///
/// In this case this is relatively simple but there's no restrictions on what
/// this type can hold other than that it must be `'static + Send`.
pub struct MyLogger {
    pub max_level: example::resource_async::logging::Level,
}
