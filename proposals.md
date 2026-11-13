1. CallContext optimization

    When the current_scope is a DeferredHost, try inlining the actuall
    CallContext directly so that it doesn't need to be looked up each time. This
    field might need to be behind a reference, Box, or Arc. Consider all options
    and determine if any have a noticeable impact on the p3_http_proxy
    benchmark.

2. set_thread optimization

    Make an optimized version of set_thread which is only to be used in the sync
    context when we can ensure nothing will access the context slots. In these
    cases we can skip setting the context.

3. Object storage

    Attempt to store async runtime objects in typed tables as described in the
    report instead of in generic ResourceTables.
