use log::{debug, error, info};
use std::net::UdpSocket;
use std::str;
use std::sync::{Arc, Mutex};

/// 通用响应类型
pub type N2nResponse = serde_json::Value;

/// N2N 协议客户端
#[derive(Clone)]
pub struct N2nClient {
    socket: Arc<Mutex<UdpSocket>>,
    address: String,
}

impl N2nClient {
    /// 创建新的 N2N 客户端
    pub fn new(address: &str) -> Result<Self, std::io::Error> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
        Ok(Self {
            socket: Arc::new(Mutex::new(socket)),
            address: address.to_string(),
        })
    }

    /// 发送命令并接收响应
    pub fn send_command(&self, command: &str) -> Result<Vec<N2nResponse>, Box<dyn std::error::Error + '_>> {
        let address = self.address.clone();

        // 锁定 socket，确保同一时间只有一个请求
        let socket_guard = self.socket.lock()?;

        // 发送命令
        socket_guard.send_to(command.as_bytes(), &address)?;
        info!("Sent N2N command: {} to {}", command, address);

        // 接收响应
        let mut buffer = [0; 4096];
        let mut responses = Vec::new();
        let mut end_received = false;

        while !end_received {
            match socket_guard.recv_from(&mut buffer) {
                Ok((size, _)) => {
                    let response_data = str::from_utf8(&buffer[..size])?;
                    debug!("Received N2N response: {}", response_data);

                    // 处理响应数据，可能包含多个 JSON 对象
                    let mut start = 0;
                    let mut depth = 0;
                    let mut in_string = false;
                    let mut escape = false;

                    for (i, c) in response_data.char_indices() {
                        match c {
                            '"' if !escape => in_string = !in_string,
                            '\\' if in_string => escape = !escape,
                            '{' if !in_string => depth += 1,
                            '}' if !in_string => {
                                depth -= 1;
                                if depth == 0 {
                                    // 找到一个完整的 JSON 对象
                                    let json_str = &response_data[start..i+1];
                                    match serde_json::from_str(json_str) {
                                        Ok(n2n_response) => {
                                            responses.push(n2n_response);
                                            
                                            // 检查是否收到结束标记
                                            if let Some(serde_json::Value::Object(obj)) = responses.last() {
                                                if let Some(r#type) = obj.get("_type") {
                                                    if r#type.as_str() == Some("end") {
                                                        end_received = true;
                                                    }
                                                }
                                            }
                                        },
                                        Err(e) => {
                                            error!("Failed to parse JSON: {:?}, data: {}", e, json_str);
                                        }
                                    }
                                    start = i + 1;
                                }
                            },
                            _ => escape = false,
                        }
                    }
                }
                Err(e) => {
                    error!("N2N receive error: {:?}", e);
                    break;
                }
            }
        }

        Ok(responses)
    }

    /// 处理响应数据
    pub fn process_response(&self, responses: Vec<N2nResponse>, cmd: &str) -> Result<serde_json::Value, Box<dyn std::error::Error + '_>> {
        // 检查是否有 unknowncmd 错误
        for resp in &responses {
            if let serde_json::Value::Object(obj) = resp {
                if let Some(r#type) = obj.get("_type") {
                    if r#type.as_str() == Some("error") {
                        if let Some(error) = obj.get("error") {
                            if error.as_str() == Some("unknowncmd") {
                                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "not implemented")));
                            }
                        }
                    }
                }
            }
        }

        // 过滤出 _type = row 的行
        let row_responses: Vec<N2nResponse> = responses
            .into_iter()
            .filter(|resp| {
                if let Some(r#type) = resp.get("_type") {
                    r#type.as_str() == Some("row")
                } else {
                    false
                }
            })
            .collect();

        // 处理不同类型的请求
        let response_data = match cmd {
            "timestamps" => {
                // timestamp 不需要搞成列表（只留下了一行）
                if let Some(first) = row_responses.first() {
                    // 去掉下划线开头的 key
                    self.filter_out_underscore_keys(first)
                } else {
                    serde_json::Value::Null
                }
            }
            "edges" | "supernodes" | "communities" | "packetstats" | "info" => {
                // 把留下的东西组成列表返回
                let filtered_responses: Vec<serde_json::Value> = row_responses
                    .into_iter()
                    .map(|resp| self.filter_out_underscore_keys(&resp))
                    .collect();
                serde_json::Value::Array(filtered_responses)
            }
            _ => {
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid cmd")));
            }
        };

        // 包装成 {"result": ... } 格式
        let mut result_map = serde_json::Map::new();
        result_map.insert("result".to_string(), response_data);
        Ok(serde_json::Value::Object(result_map))
    }

    /// 过滤掉下划线开头的 key
    fn filter_out_underscore_keys(&self, value: &N2nResponse) -> serde_json::Value {
        if let serde_json::Value::Object(map) = value {
            let mut filtered_map = serde_json::Map::new();
            for (key, val) in map {
                if !key.starts_with('_') {
                    filtered_map.insert(key.clone(), self.filter_out_underscore_keys(val));
                }
            }
            serde_json::Value::Object(filtered_map)
        } else if let serde_json::Value::Array(array) = value {
            let filtered_array: Vec<serde_json::Value> = array
                .iter()
                .map(|item| self.filter_out_underscore_keys(item))
                .collect();
            serde_json::Value::Array(filtered_array)
        } else {
            value.clone()
        }
    }

    /// 发送命令的通用方法
    pub fn send_cmd(&self, cmd: &str) -> Result<serde_json::Value, Box<dyn std::error::Error + '_>> {
        let command = format!("r 1 {}", cmd);
        let responses = self.send_command(&command)?;
        self.process_response(responses, &cmd)
    }

    /// 返回未实现错误
    pub fn not_implemented(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + '_>> {
        Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "not implemented")))
    }
}