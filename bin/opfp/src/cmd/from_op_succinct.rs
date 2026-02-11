//! From OP Succinct Subcommand
//!
//! Generates an SP1Stdin fixture by fetching witness data through the
//! OP Succinct host pipeline (ETH-DA / SingleChainOPSuccinctHost).

use clap::{ArgAction, Parser};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use op_succinct_ethereum_host_utils::host::SingleChainOPSuccinctHost;
use op_succinct_host_utils::fetcher::OPSuccinctDataFetcher;
use op_succinct_host_utils::host::OPSuccinctHost;
use op_succinct_host_utils::witness_generation::WitnessGenerator;
use sp1_sdk::SP1Stdin;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, trace};

/// The logging target to use for [tracing].
const TARGET: &str = "from-op-succinct";

/// CLI arguments for the `from-op-succinct` subcommand of `opfp`.
///
/// Requires the following environment variables:
///   - `L1_RPC`        — L1 execution RPC endpoint
///   - `L1_BEACON_RPC` — L1 beacon (consensus) RPC endpoint
///   - `L2_RPC`        — L2 execution RPC endpoint
///   - `L2_NODE_RPC`   — L2 rollup node RPC endpoint
#[derive(Parser, Clone, Debug)]
pub struct FromOpSuccinct {
    /// The L2 start block (inclusive). Typically `end_block - 1`.
    #[clap(long, help = "L2 start block number (inclusive)")]
    pub l2_start_block: u64,
    /// The L2 end block (inclusive) — the block whose execution we benchmark.
    #[clap(long, help = "L2 end block number (inclusive)")]
    pub l2_end_block: u64,
    /// The output file path for the serialized SP1Stdin fixture.
    #[clap(long, help = "Output file for the SP1Stdin fixture (JSON)")]
    pub output: PathBuf,
    /// Verbosity level (0-4)
    #[arg(long, short, help = "Verbosity level (0-4)", action = ArgAction::Count)]
    pub v: u8,
}

impl FromOpSuccinct {
    /// Runs the from-op-succinct subcommand.
    ///
    /// 1. Creates an `OPSuccinctDataFetcher` from environment variables.
    /// 2. Wraps it in a `SingleChainOPSuccinctHost`.
    /// 3. Fetches host args for the given block range.
    /// 4. Generates the witness by running the host.
    /// 5. Converts the witness into `SP1Stdin`.
    /// 6. Serializes the `SP1Stdin` to JSON at the output path.
    pub async fn run(&self) -> Result<()> {
        trace!(target: TARGET, "Generating OP Succinct fixture for L2 blocks {} -> {}",
            self.l2_start_block, self.l2_end_block);

        // 1. Initialize the data fetcher from env vars (L1_RPC, L1_BEACON_RPC, L2_RPC, L2_NODE_RPC).
        info!(target: TARGET, "Initializing OPSuccinctDataFetcher from environment...");
        let fetcher = OPSuccinctDataFetcher::new_with_rollup_config()
            .await
            .map_err(|e| eyre!("Failed to create OPSuccinctDataFetcher: {}", e))?;

        // 2. Create the SingleChainOPSuccinctHost (ETH-DA).
        let host = SingleChainOPSuccinctHost::new(Arc::new(fetcher));

        // 3. Fetch host arguments for the block range.
        info!(target: TARGET, "Fetching host args for blocks {} -> {}...",
            self.l2_start_block, self.l2_end_block);
        let host_args = host
            .fetch(self.l2_start_block, self.l2_end_block, None, true)
            .await
            .map_err(|e| eyre!("Failed to fetch host args: {}", e))?;

        // 4. Run the host to generate witness data.
        info!(target: TARGET, "Running host to generate witness...");
        let witness = host
            .run(&host_args)
            .await
            .map_err(|e| eyre!("Failed to run host: {}", e))?;

        // 5. Convert the witness into SP1Stdin.
        info!(target: TARGET, "Converting witness to SP1Stdin...");
        let sp1_stdin: SP1Stdin = host
            .witness_generator()
            .get_sp1_stdin(witness)
            .map_err(|e| eyre!("Failed to create SP1Stdin: {}", e))?;

        // 6. Serialize SP1Stdin to JSON and write to output file.
        if let Some(parent) = self.output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string(&sp1_stdin)
            .map_err(|e| eyre!("Failed to serialize SP1Stdin: {}", e))?;
        std::fs::write(&self.output, json)?;

        info!(target: TARGET, "Wrote SP1Stdin fixture to {:?}", self.output);
        Ok(())
    }
}
