use reqwest::Client;
use serde::{Deserialize, Serialize};

/// JSON-RPC 请求结构
#[derive(Serialize, Deserialize)]
pub struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    id: u32,
}

/// JSON-RPC 响应结构
#[derive(Serialize, Deserialize)]
pub struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
    id: serde_json::Value,
}

/// API 响应结构
#[derive(Serialize)]
pub struct ApiResponse {
    result: Option<serde_json::Value>,
}

/// N3N 协议客户端
#[derive(Clone)]
pub struct N3nClient {
    client: Client,
    api_endpoint: String,
}

impl N3nClient {
    /// 创建新的 N3N 客户端
    pub fn new(api_endpoint: &str) -> Result<Self, reqwest::Error> {
        let using_unix_socket = api_endpoint.starts_with("unix://");
        let using_http = api_endpoint.starts_with("http://") || api_endpoint.starts_with("https://");
        
        let client = if using_unix_socket {
            #[cfg(unix)]
            {
                let socket_path = api_endpoint.trim_start_matches("unix://");
                Client::builder()
                    .unix_socket(socket_path)
                    .build()?
            }
            #[cfg(not(unix))]
            {
                Client::new()
            }
        } else if using_http {
            // 对于 HTTP/HTTPS 端点，直接使用默认客户端
            Client::new()
        } else {
            Client::new()
        };
        
        Ok(Self {
            client,
            api_endpoint: api_endpoint.to_string(),
        })
    }

    /// 发送 JSON-RPC 请求
    pub async fn send_request(&self, method: &str) -> Result<ApiResponse, Box<dyn std::error::Error>> {
        let payload = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            id: 1,
        };

        let response = if self.api_endpoint.starts_with("unix://") {
            // 对于 Unix 域套接字，使用 http://x/v1 格式
            self.client.post("http://x/v1").json(&payload).send().await?
        } else {
            // 对于 HTTP/HTTPS，使用完整的 URL
            let url = format!("{}/v1", self.api_endpoint.trim_end_matches('/'));
            self.client.post(&url).json(&payload).send().await?
        };

        let response_data: JsonRpcResponse = response.json().await?;

        Ok(ApiResponse {
            result: response_data.result,
        })
    }
}