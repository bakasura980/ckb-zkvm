//! CKB RPC client for fetching transaction and cell data.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use ckb_jsonrpc_types::{
    BlockView, CellWithStatus, HeaderView, OutPoint, TransactionWithStatusResponse,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{debug, instrument};

/// CKB RPC client configuration.
#[derive(Clone, Debug)]
pub struct RpcConfig {
    /// RPC endpoint URL.
    pub url: String,
    /// Request timeout.
    pub timeout: Duration,
    /// Maximum retry attempts.
    pub max_retries: u32,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            url: "https://testnet.ckb.dev/rpc".to_string(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
        }
    }
}

/// CKB JSON-RPC client.
pub struct CkbRpcClient {
    client: Client,
    config: RpcConfig,
}

/// JSON-RPC request wrapper.
#[derive(Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'static str,
    method: &'a str,
    params: Value,
    id: u64,
}

/// JSON-RPC response wrapper.
#[derive(Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcError>,
}

/// JSON-RPC batch response item.
#[derive(Deserialize)]
struct BatchRpcResponseItem<T> {
    id: u64,
    result: Option<T>,
    error: Option<RpcError>,
}

/// JSON-RPC error.
#[derive(Deserialize, Debug)]
struct RpcError {
    code: i64,
    message: String,
}

impl CkbRpcClient {
    /// Create a new RPC client with the given configuration.
    pub fn new(config: RpcConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .context("failed to create HTTP client")?;

        Ok(Self { client, config })
    }

    /// Create a new RPC client with default configuration.
    pub fn with_url(url: impl Into<String>) -> Result<Self> {
        Self::new(RpcConfig {
            url: url.into(),
            ..Default::default()
        })
    }

    /// Make an RPC call with retries.
    #[instrument(skip(self, params))]
    async fn call<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<T> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                debug!("retry attempt {} for {}", attempt, method);
                tokio::time::sleep(Duration::from_millis(100 * (1 << attempt))).await;
            }

            let request = RpcRequest {
                jsonrpc: "2.0",
                method,
                params: params.clone(),
                id: 1,
            };

            match self.execute_request::<T>(&request).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("RPC call failed")))
    }

    /// Execute a single RPC request.
    async fn execute_request<T: for<'de> Deserialize<'de>>(
        &self,
        request: &RpcRequest<'_>,
    ) -> Result<T> {
        let response = self
            .client
            .post(&self.config.url)
            .json(request)
            .send()
            .await
            .context("failed to send RPC request")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "RPC request failed with status: {}",
                response.status()
            ));
        }

        let rpc_response: RpcResponse<T> = response
            .json()
            .await
            .context("failed to parse RPC response")?;

        if let Some(error) = rpc_response.error {
            return Err(anyhow!(
                "RPC error {}: {}",
                error.code,
                error.message
            ));
        }

        rpc_response
            .result
            .ok_or_else(|| anyhow!("RPC response missing result"))
    }

    /// Fetch a transaction by hash.
    #[instrument(skip(self))]
    pub async fn get_transaction(
        &self,
        tx_hash: &str,
    ) -> Result<TransactionWithStatusResponse> {
        debug!("fetching transaction {}", tx_hash);

        self.call("get_transaction", json!([tx_hash]))
            .await
            .with_context(|| format!("failed to fetch transaction {}", tx_hash))
    }

    /// Fetch a live cell by out_point.
    #[instrument(skip(self))]
    pub async fn get_live_cell(
        &self,
        out_point: OutPoint,
        with_data: bool,
    ) -> Result<CellWithStatus> {
        debug!("fetching cell {:?}", out_point);

        self.call("get_live_cell", json!([out_point, with_data]))
            .await
            .with_context(|| format!("failed to fetch cell {:?}", out_point))
    }

    /// Fetch a block by hash.
    #[instrument(skip(self))]
    pub async fn get_block(&self, block_hash: &str) -> Result<BlockView> {
        debug!("fetching block {}", block_hash);

        self.call("get_block", json!([block_hash]))
            .await
            .with_context(|| format!("failed to fetch block {}", block_hash))
    }

    /// Fetch a block by number.
    #[instrument(skip(self))]
    pub async fn get_block_by_number(&self, block_number: u64) -> Result<BlockView> {
        let hex_number = format!("0x{:x}", block_number);
        debug!("fetching block #{}", block_number);

        self.call("get_block_by_number", json!([hex_number]))
            .await
            .with_context(|| format!("failed to fetch block #{}", block_number))
    }

    /// Get the current tip (highest) block number.
    pub async fn get_tip_block_number(&self) -> Result<u64> {
        let hex: String = self
            .call("get_tip_block_number", json!([]))
            .await
            .context("failed to fetch tip block number")?;

        u64::from_str_radix(hex.trim_start_matches("0x"), 16)
            .context("invalid tip block number format")
    }

    /// Execute a JSON-RPC batch request with retries.
    ///
    /// Sends an array of JSON-RPC request objects in one HTTP POST.
    /// The server returns an array of response objects (possibly out of order),
    /// which are sorted by id and returned in the original request order.
    /// Uses the same exponential backoff retry logic as single RPC calls.
    async fn execute_batch<T: for<'de> Deserialize<'de>>(
        &self,
        requests: &[Value],
        label: &str,
    ) -> Result<Vec<T>> {
        let count = requests.len();
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                debug!("retry attempt {} for batch {} ({})", attempt, label, count);
                tokio::time::sleep(Duration::from_millis(100 * (1 << attempt))).await;
            }

            match self.execute_batch_attempt::<T>(requests, label).await {
                Ok(results) => return Ok(results),
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("batch {} RPC call failed", label)))
    }

    /// Execute a single batch attempt (no retries).
    async fn execute_batch_attempt<T: for<'de> Deserialize<'de>>(
        &self,
        requests: &[Value],
        label: &str,
    ) -> Result<Vec<T>> {
        let response = self
            .client
            .post(&self.config.url)
            .json(requests)
            .send()
            .await
            .with_context(|| format!("failed to send batch {} RPC request", label))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "batch {} RPC request failed with status: {}",
                label,
                response.status()
            ));
        }

        let mut batch_responses: Vec<BatchRpcResponseItem<T>> = response
            .json()
            .await
            .with_context(|| format!("failed to parse batch {} RPC response", label))?;

        // Sort by id to restore original order
        batch_responses.sort_by_key(|r| r.id);

        let mut results = Vec::with_capacity(requests.len());
        for (i, item) in batch_responses.into_iter().enumerate() {
            if let Some(error) = item.error {
                return Err(anyhow!(
                    "batch {} RPC error for item {}: {} (code {})",
                    label,
                    i,
                    error.message,
                    error.code
                ));
            }
            let result = item.result.ok_or_else(|| {
                anyhow!("batch {} RPC response missing result for item {}", label, i)
            })?;
            results.push(result);
        }

        Ok(results)
    }

    /// Fetch multiple transactions in a single JSON-RPC batch request.
    ///
    /// JSON-RPC 2.0 supports sending an array of request objects in one HTTP POST.
    /// The server returns an array of response objects (possibly out of order).
    /// Results are returned in the same order as the input `tx_hashes`.
    pub async fn batch_get_transactions(
        &self,
        tx_hashes: &[String],
    ) -> Result<Vec<TransactionWithStatusResponse>> {
        if tx_hashes.is_empty() {
            return Ok(Vec::new());
        }

        debug!("batch fetching {} transactions", tx_hashes.len());

        let requests: Vec<Value> = tx_hashes
            .iter()
            .enumerate()
            .map(|(i, hash)| {
                json!({
                    "jsonrpc": "2.0",
                    "method": "get_transaction",
                    "params": [hash],
                    "id": i as u64
                })
            })
            .collect();

        self.execute_batch(&requests, "get_transaction").await
    }

    /// Fetch multiple live cells in a single JSON-RPC batch request.
    ///
    /// Returns results in the same order as the input `out_points`.
    /// Individual failures return `None` in the cell field (not an error).
    pub async fn batch_get_live_cells(
        &self,
        out_points: &[OutPoint],
        with_data: bool,
    ) -> Result<Vec<CellWithStatus>> {
        if out_points.is_empty() {
            return Ok(Vec::new());
        }

        debug!("batch fetching {} live cells", out_points.len());

        let requests: Vec<Value> = out_points
            .iter()
            .enumerate()
            .map(|(i, op)| {
                json!({
                    "jsonrpc": "2.0",
                    "method": "get_live_cell",
                    "params": [op, with_data],
                    "id": i as u64
                })
            })
            .collect();

        self.execute_batch(&requests, "get_live_cell").await
    }

    /// Fetch multiple block headers in a single JSON-RPC batch request.
    ///
    /// Uses the `get_header` RPC method. Returns results in the same order
    /// as the input `block_hashes`.
    pub async fn batch_get_headers(
        &self,
        block_hashes: &[String],
    ) -> Result<Vec<HeaderView>> {
        if block_hashes.is_empty() {
            return Ok(Vec::new());
        }

        debug!("batch fetching {} headers", block_hashes.len());

        let requests: Vec<Value> = block_hashes
            .iter()
            .enumerate()
            .map(|(i, hash)| {
                json!({
                    "jsonrpc": "2.0",
                    "method": "get_header",
                    "params": [hash],
                    "id": i as u64
                })
            })
            .collect();

        self.execute_batch(&requests, "get_header").await
    }

    /// Check if a transaction exists.
    pub async fn tx_exists(&self, tx_hash: &str) -> Result<bool> {
        match self.get_transaction(tx_hash).await {
            Ok(resp) => Ok(resp.transaction.is_some()),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RpcConfig::default();
        assert_eq!(config.url, "https://testnet.ckb.dev/rpc");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 3);
    }
}
