use rust_playground::simple_http_server::start_server_async;

#[async_std::main]
async fn main() {
    println!("== simple http server async ==");
    start_server_async().await;
}
