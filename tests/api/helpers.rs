use once_cell::sync::Lazy;
use reqwest::Url;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::get_configuration,
    startup::{configuration_database, AppState},
    telemetry::init,
};

static TRACING: Lazy<()> = Lazy::new(|| {
    let configuration = get_configuration().expect("Failed to read configuration.");
    init(&configuration.logger);
});

pub struct TestApp {
    pub app_state: AppState,
    pub email_server: MockServer,
}

impl TestApp {
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let confirmation_link = Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link
        };
        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    configuration.email_client.base_url = email_server.uri();
    configuration_database(&configuration.database).await;
    let app_state = AppState::build(&configuration).await;
    TestApp {
        app_state,
        email_server,
    }
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub fn path_and_query(link: reqwest::Url) -> String {
    let url_path = link.path();
    let query = link.query().unwrap();
    format!("{}?{}", url_path, query)
}
