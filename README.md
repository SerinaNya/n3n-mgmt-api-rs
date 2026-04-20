# n3n-mgmt-api-rs

A Rust-based management API for n3n and n2n, providing a RESTful interface to interact with edge and supernode nodes.

## Features

- **Dual Protocol Support**: Works with both n3n (JSON-RPC over Unix socket/HTTP) and n2n (UDP) protocols
- **RESTful API**: Provides a clean RESTful interface for managing network nodes
- **Static File Server**: Serves a web-based management interface from the `dist` directory
- **Health Check Endpoint**: Includes a `/health` endpoint for monitoring
- **Command Line Configuration**: Supports configuration via command line arguments

## Installation

### Prerequisites

- Rust 1.68.0 or later
- Cargo (Rust package manager)

### Build from Source

```bash
git clone https://github.com/yourusername/n3n-mgmt-api-rs.git
cd n3n-mgmt-api-rs
cargo build --release
```

## Usage

### Command Line Arguments

```bash
n3n-mgmt-api-rs [OPTIONS]

Options:
  --api-endpoint <API_ENDPOINT>  API endpoint URL [default: unix:///run/n3n/edge/mgmt]
  --host <HOST>                  Server host [default: 0.0.0.0]
  --port <PORT>                  Server port [default: 8376]
  --help                         Print help information
  --version                      Print version information
```

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/edges` | GET | Get edge node information |
| `/api/supernodes` | GET | Get supernode information |
| `/api/info` | GET | Get system information (not implemented in n2n) |
| `/api/packetstats` | GET | Get packet statistics |
| `/api/timestamps` | GET | Get timestamp information |
| `/api/communities` | GET | Get community information |
| `/health` | GET | Health check endpoint |
| `/` | GET | Serve static files from the `dist` directory |

### Examples

#### Using Unix Domain Socket (n3n)

```bash
n3n-mgmt-api-rs --api-endpoint unix:///run/n3n/edge/mgmt
```

#### Using HTTP Endpoint (n3n)

```bash
n3n-mgmt-api-rs --api-endpoint http://localhost:7777
```

#### Using UDP Endpoint (n2n)

```bash
n3n-mgmt-api-rs --api-endpoint udp://127.0.0.1:5644
```

## Project Structure

```
n3n-mgmt-api-rs/
├── dist/              # Static files for web interface
├── docs/              # Documentation
├── src/               # Source code
│   ├── main.rs        # Main entry point
│   ├── n2n_protocol.rs # N2N protocol implementation
│   └── n3n_protocol.rs # N3N protocol implementation
├── Cargo.toml         # Rust package configuration
└── README.md          # This file
```

## Protocol Support

### N3N Protocol

- Uses JSON-RPC over Unix domain socket or HTTP
- Supports commands: `get_edges`, `get_supernodes`, `get_info`, `get_packetstats`, `get_timestamps`, `get_communities`

### N2N Protocol

- Uses UDP for communication
- Supports commands: `get_edges`, `get_supernodes`, `get_packetstats`, `get_timestamps`, `get_communities`
- Note: `get_info` is not implemented in n2n

## License

[MIT](LICENSE)
