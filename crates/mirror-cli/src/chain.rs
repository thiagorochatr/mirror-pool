//! The few RPC calls a live run needs, over the same blocking client the
//! provenance collector uses.
//!
//! `solana-client` is deliberately not a dependency. It pulls
//! `solana-transaction-status-client-types`, which sits on the far side of the
//! ecosystem's in-progress `wincode` migration and cannot coexist with the
//! version litesvm requires. Rather than fight that resolution a second time,
//! this speaks JSON-RPC directly — four methods, all of which we already needed
//! to understand.

use anyhow::{anyhow, Result};
use base64::Engine;
use solana_program::pubkey::Pubkey;
use solana_transaction::Transaction;

pub struct Chain {
    endpoint: String,
    agent: ureq::Agent,
}

impl Chain {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Chain {
            endpoint: endpoint.into(),
            agent: ureq::AgentBuilder::new()
                .timeout(std::time::Duration::from_secs(60))
                .build(),
        }
    }

    /// One call, retrying through rate limits.
    ///
    /// The public endpoints throttle aggressively and a 429 is not a failure of
    /// the thing being measured — treating it as one would abandon a run that is
    /// otherwise fine. Backoff is exponential and bounded, and a persistent 429
    /// still surfaces as an error rather than as a silent default.
    fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": method, "params": params,
        });
        let mut wait = std::time::Duration::from_millis(500);
        let mut last: Option<String> = None;
        let mut response: Option<serde_json::Value> = None;
        for _ in 0..7 {
            match self.agent.post(&self.endpoint).send_json(body.clone()) {
                Ok(r) => {
                    response = Some(r.into_json()?);
                    break;
                }
                Err(ureq::Error::Status(429, _)) => {
                    std::thread::sleep(wait);
                    wait *= 2;
                    last = Some("rate limited".into());
                }
                Err(e) => return Err(anyhow!("{method}: {e}")),
            }
        }
        let response = response.ok_or_else(|| {
            anyhow!(
                "{method}: gave up after retries ({})",
                last.unwrap_or_default()
            )
        })?;
        if let Some(err) = response.get("error") {
            return Err(anyhow!("{method}: {err}"));
        }
        response
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow!("{method}: no result"))
    }

    pub fn latest_blockhash(&self) -> Result<solana_program::hash::Hash> {
        let v = self.call(
            "getLatestBlockhash",
            serde_json::json!([{ "commitment": "confirmed" }]),
        )?;
        let s = v
            .pointer("/value/blockhash")
            .and_then(|b| b.as_str())
            .ok_or_else(|| anyhow!("blockhash missing"))?;
        s.parse().map_err(|e| anyhow!("bad blockhash: {e}"))
    }

    /// Sends and waits for confirmation, returning the signature.
    ///
    /// A failed simulation is surfaced with its logs rather than a bare error,
    /// because the program's own error code is the thing worth recording.
    pub fn send(&self, tx: &Transaction) -> Result<String> {
        let wire = bincode::serialize(tx)?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&wire);
        let sig = self.call(
            "sendTransaction",
            serde_json::json!([
                encoded,
                { "encoding": "base64", "preflightCommitment": "confirmed" }
            ]),
        )?;
        let sig = sig
            .as_str()
            .ok_or_else(|| anyhow!("signature missing"))?
            .to_string();
        self.confirm(&sig)?;
        Ok(sig)
    }

    fn confirm(&self, signature: &str) -> Result<()> {
        for _ in 0..40 {
            let v = self.call(
                "getSignatureStatuses",
                serde_json::json!([[signature], { "searchTransactionHistory": true }]),
            )?;
            if let Some(status) = v.pointer("/value/0") {
                if !status.is_null() {
                    if let Some(err) = status.get("err") {
                        if !err.is_null() {
                            return Err(anyhow!("transaction failed: {err}"));
                        }
                    }
                    let confirmed = status
                        .get("confirmationStatus")
                        .and_then(|c| c.as_str())
                        .unwrap_or("");
                    if confirmed == "confirmed" || confirmed == "finalized" {
                        return Ok(());
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(1_200));
        }
        Err(anyhow!("timed out waiting for {signature}"))
    }

    pub fn account_data(&self, key: &Pubkey) -> Result<Option<Vec<u8>>> {
        let v = self.call(
            "getAccountInfo",
            // Commitment must be stated. The default is finalized, so a read
            // immediately after a confirmed write sees nothing and the caller
            // concludes the account was never created.
            serde_json::json!([
                key.to_string(),
                { "encoding": "base64", "commitment": "confirmed" }
            ]),
        )?;
        let value = v.get("value").unwrap_or(&serde_json::Value::Null);
        if value.is_null() {
            return Ok(None);
        }
        let b64 = value
            .pointer("/data/0")
            .and_then(|d| d.as_str())
            .ok_or_else(|| anyhow!("account data missing"))?;
        Ok(Some(base64::engine::general_purpose::STANDARD.decode(b64)?))
    }

    pub fn balance(&self, key: &Pubkey) -> Result<u64> {
        let v = self.call(
            "getBalance",
            serde_json::json!([key.to_string(), { "commitment": "confirmed" }]),
        )?;
        v.pointer("/value")
            .and_then(|b| b.as_u64())
            .ok_or_else(|| anyhow!("balance missing"))
    }

    /// The rent-exempt floor for an account of `space` bytes.
    ///
    /// Asked of the cluster rather than computed here. The rent parameters are
    /// chain state, so a constant baked into this binary would be a second
    /// source of truth that is right until it is not — and the number is used
    /// to assert that a vault settled to its floor exactly, which is a claim
    /// worth grounding in what the cluster itself says.
    pub fn rent_exempt_minimum(&self, space: usize) -> Result<u64> {
        let v = self.call(
            "getMinimumBalanceForRentExemption",
            serde_json::json!([space, { "commitment": "confirmed" }]),
        )?;
        v.as_u64()
            .ok_or_else(|| anyhow!("rent-exempt minimum missing"))
    }
}
