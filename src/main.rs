use zero2prod::{
    configuration::get_configuration,
    startup::{run_until_stopped, AppState},
    telemetry::init_tracing,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_tracing();
    let configuration = get_configuration().expect("Failed to read configuration.");
    let app_state = AppState::build(&configuration).await;
    run_until_stopped(app_state, configuration).await
}
