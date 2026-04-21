use actix_web::{
    middleware::{Compress, Logger},
    web, App, HttpRequest, HttpResponse, HttpServer, Result,
};
use clap::Parser;
use log::{error, info};
use std::path::PathBuf;

// 导入协议模块
mod n2n_protocol;
mod n3n_protocol;

use n2n_protocol::N2nClient;
use n3n_protocol::N3nClient;

/// 命令行参数
#[derive(Parser, Debug)]
#[command(author, version, about,  long_about = None)]
struct Args {
    #[arg(
        long,
        default_value = "unix:///run/n3n/edge/mgmt",
        help = "
        n3n / n2nManagementAPI endpoint URL

        - For n3n: `unix:///run/n3n/edge/mgmt` or `http://127.0.0.1:{management_port}`
        - For n2n: `udp://127.0.0.1:5644`
       "
    )]
    api_endpoint: String,

    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    #[arg(long, default_value_t = 8376)]
    port: u16,
}

/// 协议类型
#[derive(Clone)]
pub enum ProtocolType {
    N3n(N3nClient),
    N2n(N2nClient),
}

/// 应用配置
#[derive(Clone)]
pub struct AppConfig {
    pub api_endpoint: String,
    pub host: String,
    pub port: u16,
}

/// 处理请求的通用函数
///
/// # 参数
/// - `protocol`: 协议客户端
/// - `api_endpoint`: API 端点 URL
/// - `method`: 要调用的方法名
///
/// # 返回值
/// - `Ok(HttpResponse)`: 成功时返回 HTTP 响应
/// - `Err(Error)`: 失败时返回错误
async fn handle_request(
    protocol: web::Data<ProtocolType>,
    _api_endpoint: web::Data<String>,
    method: &str,
) -> Result<HttpResponse> {
    match &**protocol {
        ProtocolType::N3n(client) => {
            // 使用 send_cmd 方法调用 N3N 客户端方法
            let result = match method {
                "get_edges" => client.send_request(method).await,
                "get_supernodes" => client.send_request(method).await,
                "get_info" => client.send_request(method).await,
                "get_packetstats" => client.send_request(method).await,
                "get_timestamps" => client.send_request(method).await,
                "get_communities" => client.send_request(method).await,
                _ => Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid method",
                )) as Box<dyn std::error::Error>),
            };

            match result {
                Ok(response) => Ok(HttpResponse::Ok().json(response)),
                Err(e) => {
                    error!("N3N request failed: {:?}", e);
                    Ok(HttpResponse::InternalServerError()
                        .body(format!("N3N request failed: {:?}", e)))
                }
            }
        }
        ProtocolType::N2n(client) => {
            // 使用 send_cmd 方法调用 N2N 客户端方法
            let result = match method {
                "get_edges" => client.send_cmd("edges"),
                "get_supernodes" => client.send_cmd("supernodes"),
                "get_info" => client.send_cmd("info"),
                "get_packetstats" => client.send_cmd("packetstats"),
                "get_timestamps" => client.send_cmd("timestamps"),
                "get_communities" => client.send_cmd("communities"),
                _ => Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid method",
                )) as Box<dyn std::error::Error + '_>),
            };

            match result {
                Ok(response) => Ok(HttpResponse::Ok().json(response)),
                Err(e) => {
                    if e.to_string().contains("not implemented") {
                        // info 直接报错 404，因为没实现
                        Ok(HttpResponse::NotFound().body("not implemented"))
                    } else {
                        error!("N2N request failed: {:?}", e);
                        Ok(HttpResponse::InternalServerError()
                            .body(format!("N2N request failed: {:?}", e)))
                    }
                }
            }
        }
    }
}

/// 通用 API 处理函数
async fn api_handler(
    protocol: web::Data<ProtocolType>,
    api_endpoint: web::Data<String>,
    method: web::Path<String>,
) -> Result<HttpResponse> {
    // 验证 method 是否在允许的列表中
    let allowed_methods = [
        "edges",
        "supernodes",
        "info",
        "packetstats",
        "timestamps",
        "communities",
    ];
    if !allowed_methods.contains(&method.as_str()) {
        return Ok(HttpResponse::BadRequest().body("Invalid method"));
    }

    let method_name = format!("get_{}", method);
    handle_request(protocol, api_endpoint, &method_name).await
}

/// 健康检查端点
async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body("ok"))
}

/// 提供静态文件服务
async fn serve_static(req: HttpRequest) -> Result<actix_files::NamedFile> {
    let path: PathBuf = req.match_info().query("filename").parse()?;
    let mut file_path = PathBuf::from("./dist");
    file_path.push(path);

    // 支持 cleanUrl：检查路径是否为目录，如果是则尝试添加 index.html
    if file_path.is_dir() {
        file_path.push("index.html");
        if file_path.exists() {
            return Ok(actix_files::NamedFile::open(file_path)?);
        }
    }

    // 检查文件是否存在
    if file_path.exists() {
        Ok(actix_files::NamedFile::open(file_path)?)
    } else {
        // 对于不存在的文件，返回 index.html，支持单页应用路由
        // 这确保了 SPA 应用的前端路由能够正常工作
        Ok(actix_files::NamedFile::open("./dist/index.html")?)
    }
}

/// 初始化协议客户端
fn init_protocol(api_endpoint: &str) -> Result<ProtocolType, std::io::Error> {
    if api_endpoint.starts_with("unix://")
        || api_endpoint.starts_with("http://")
        || api_endpoint.starts_with("https://")
    {
        // N3N 协议 (JSON-RPC)
        let using_unix_socket = api_endpoint.starts_with("unix://");
        let using_http =
            api_endpoint.starts_with("http://") || api_endpoint.starts_with("https://");

        let client = match N3nClient::new(api_endpoint) {
            Ok(client) => client,
            Err(e) => {
                error!("Failed to create N3N client: {:?}", e);
                return Err(e);
            }
        };

        info!(
            "n3n management endpoint: {}, using unix domain socket: {}, using http: {}",
            api_endpoint, using_unix_socket, using_http
        );

        Ok(ProtocolType::N3n(client))
    } else if api_endpoint.starts_with("udp://") {
        // N2N 协议 (UDP)
        let n2n_address = api_endpoint.trim_start_matches("udp://");
        match N2nClient::new(n2n_address) {
            Ok(client) => {
                info!("n2n management UDP endpoint: {}", api_endpoint);
                Ok(ProtocolType::N2n(client))
            }
            Err(e) => {
                error!("Failed to create N2N client: {:?}", e);
                Err(e)
            }
        }
    } else {
        error!("Unsupported endpoint format: {}", api_endpoint);
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Unsupported endpoint format",
        ))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 解析命令行参数
    let args = Args::parse();

    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // 配置 API 端点
    let api_endpoint = args.api_endpoint;

    // 初始化协议客户端
    let protocol = init_protocol(&api_endpoint)?;

    // 创建数据共享
    let protocol_data = web::Data::new(protocol);
    let api_endpoint_data = web::Data::new(api_endpoint.clone());

    // 启动服务器
    let host = args.host;
    let port = args.port;
    info!("Starting server on {} port {}", host, port);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(Compress::default())
            .app_data(protocol_data.clone())
            .app_data(api_endpoint_data.clone())
            // API 路由
            .route("/api/{method}", web::get().to(api_handler))
            // 健康检查端点
            .route("/health", web::get().to(health_check))
            // 静态文件服务
            .route("/{filename:.*}", web::get().to(serve_static))
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}
