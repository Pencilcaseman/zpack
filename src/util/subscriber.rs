#[cfg(debug_assertions)]
pub fn subscriber() -> impl tracing::Subscriber {
    // Debug implementation

    tracing_subscriber::fmt()
        .pretty()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(true)
        .finish()
}

#[cfg(not(debug_assertions))]
pub fn subscriber() -> impl tracing::Subscriber {
    // Release implementation

    tracing_subscriber::fmt()
        .compact()
        .without_time()
        .with_file(false)
        .with_line_number(false)
        .with_thread_ids(false)
        .with_target(true)
        .finish()
}
