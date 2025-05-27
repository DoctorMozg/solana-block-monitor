# Solana Block Monitor

A high-performance Solana block monitoring service that provides an HTTP API to check if slots are confirmed on the Solana blockchain.

## Features

- **Fast Slot Confirmation Checks**: Check if a specific slot is confirmed on the Solana blockchain
- **Intelligent Caching**: Uses an in-memory cache to avoid redundant RPC calls
- **Syndica RPC Integration**: Leverages Syndica's high-performance RPC endpoints
- **Configurable Logging**: Supports multiple log levels for debugging and monitoring

## API Endpoints

### GET `/isSlotConfirmed/{slot}`

Checks if a specific slot is confirmed on the Solana blockchain.

**Parameters:**

- `slot` (u64): The slot number to check

**Responses:**

- `200 OK`: Slot is confirmed
- `404 Not Found`: Slot is not confirmed
- `500 Internal Server Error`: RPC error occurred

**Example:**

```bash
curl http://localhost:3000/isSlotConfirmed/12345
```

## How It Works

1. **Cache Check**: First checks if the slot is already cached as confirmed
2. **RPC Call**: If not in cache, makes a `getBlocks` RPC call to the Syndica endpoint
3. **Cache Update**: If confirmed, adds the slot to the cache for future requests
4. **Response**: Returns appropriate HTTP status code

## Running the Server

1. **Install Rust** (if not already installed):

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Clone and setup**:

   ```bash
   git clone <repository-url>
   cd solana-block-monitor
   ```

3. **Configure environment**:

   ```bash
   cp .env.example .env
   # Edit .env with your Syndica RPC URL and other settings
   ```

4. **Run the server**:

   ```bash
   cargo run
   ```

The server will start on the configured port (default: 3000).

## Testing

Run the test suite:

```bash
cargo test
```

## Performance

- **Cache Hit**: Sub-millisecond response time
- **Cache Miss**: Response time depends on RPC latency (typically 10-100ms)
- **Memory Usage**: Configurable cache size with LRU eviction

## Dependencies

- **axum**: Modern web framework for Rust
- **tokio**: Async runtime
- **solana-client**: Official Solana RPC client
- **scc**: High-performance concurrent hash cache
- **tracing**: Structured logging

## License

[Add your license here]
