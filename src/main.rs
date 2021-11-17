use axum::{
    error_handling::HandleErrorExt,
    extract::Extension,
    http::{uri::Uri, Request, Response, StatusCode},
    routing::any,
    AddExtensionLayer, Router,
};
use chrono::Local;
use dotenv::dotenv;
use hyper::{client::HttpConnector, Body};
use pretty::termcolor::{Color, ColorChoice, ColorSpec, StandardStream};
use pretty::{Arena, DocAllocator};
use std::{convert::TryFrom, env, io};
use tower_http::{compression::CompressionLayer, services::ServeDir};

type Client = hyper::client::Client<HttpConnector, Body>;

#[tokio::main]
async fn main() {
    // 构建config参数
    dotenv().ok();
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "INFO,tower_http=DEBUG")
    }

    tracing_subscriber::fmt::init();

    let port = env::var("PORT").expect("缺少 -> 环境变量 -> PORT");
    let proxy_path = env::var("PROXY_PATH").expect("缺少 -> 环境变量 -> PROXY_PATH");
    let static_dir = env::var("STATIC_DIR").expect("缺少 -> 环境变量 -> STATIC_DIR");

    let client = Client::new();

    let assets = ServeDir::new(static_dir).handle_error(|error: io::Error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled internal error: {}", error),
        )
    });

    let app = Router::new()
        .nest("/api", any(handler))
        .layer(AddExtensionLayer::new(client))
        .fallback(assets)
        .layer(CompressionLayer::new());

    // let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let addr = format!("127.0.0.1:{}", port);
    println!("start at http://127.0.0.1:{}", port);
    println!("proxy at {}", &proxy_path);
    axum::Server::bind(&addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

/**
 * 处理代理
 * 截取调api
 */
async fn handler(Extension(client): Extension<Client>, mut req: Request<Body>) -> Response<Body> {
    let proxy_path = env::var("PROXY_PATH").unwrap();
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    let uri = format!("{}{}", proxy_path, path_query);
    print_log(&uri);
    *req.uri_mut() = Uri::try_from(uri).unwrap();

    client.request(req).await.unwrap()
}

// 打印日志
fn print_log(uri: &str) {
    let arena = Arena::new();
    let now = format!("[{}]", Local::now().format("%Y-%m-%d %H:%M:%S%.3f"));

    let proxy = arena.text(" proxy ").annotate(
        ColorSpec::new()
            .set_bg(Some(Color::Blue))
            .set_fg(Some(Color::White))
            .clone(),
    );

    let date_time = arena
        .text(now)
        .annotate(ColorSpec::new().set_fg(Some(Color::Green)).clone());

    let content = arena
        .text(format!(" {} ", uri))
        .annotate(ColorSpec::new().set_fg(Some(Color::Cyan)).clone());

    proxy
        .append(date_time)
        .append(content)
        .group()
        .1
        .render_colored(80, StandardStream::stdout(ColorChoice::Auto))
        .unwrap();
    println!();
}
