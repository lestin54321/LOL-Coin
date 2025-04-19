use clap::{App, Arg};
use solana_validator::validator::ValidatorConfig;
use std::process;

fn main() {
    let matches = App::new("Solana Validator")
        .version("1.0")
        .author("Solana Labs")
        .about("Runs a Solana validator node")
        .arg(
            Arg::new("identity")
                .long("identity")
                .takes_value(true)
                .help("Path to the identity keypair"),
        )
        .arg(
            Arg::new("vote-account")
                .long("vote-account")
                .takes_value(true)
                .help("Path to the vote account keypair"),
        )
        .arg(
            Arg::new("rpc-url")
                .long("rpc-url")
                .takes_value(true)
                .help("URL of the RPC endpoint"),
        )
        .arg(
            Arg::new("ledger")
                .long("ledger")
                .takes_value(true)
                .help("Path to the ledger directory"),
        )
        .arg(
            Arg::new("cluster")
                .long("cluster")
                .takes_value(true)
                .help("Cluster to connect to (e.g., mainnet, testnet, devnet)"),
        )
        .arg(
            Arg::new("dynamic-port-range")
                .long("dynamic-port-range")
                .takes_value(true)
                .help("Range of ports for dynamic port allocation"),
        )
        .arg(
            Arg::new("rpc-threads")
                .long("rpc-threads")
                .takes_value(true)
                .help("Number of RPC threads"),
        )
        .arg(
            Arg::new("no-telemetry")
                .long("no-telemetry")
                .takes_value(false)
                .help("Disable telemetry"),
        )
        .arg(
            Arg::new("skip-wait-for-validators")
                .long("skip-wait-for-validators")
                .takes_value(false)
                .help("Skip waiting for other validators to start"),
        )
        .get_matches();

    let config = ValidatorConfig {
        identity: matches
            .value_of("identity")
            .expect("Identity keypair is required")
            .to_string(),
        vote_account: matches
            .value_of("vote-account")
            .expect("Vote account keypair is required")
            .to_string(),
        rpc_url: matches
            .value_of("rpc-url")
            .expect("RPC URL is required")
            .to_string(),
        ledger: matches
            .value_of("ledger")
            .expect("Ledger directory is required")
            .to_string(),
        cluster: matches
            .value_of("cluster")
            .expect("Cluster is required")
            .to_string(),
        dynamic_port_range: matches
            .value_of("dynamic-port-range")
            .map(|s| s.to_string()),
        rpc_threads: matches
            .value_of("rpc-threads")
            .map(|s| s.parse().unwrap_or(4)),
        no_telemetry: matches.is_present("no-telemetry"),
        skip_wait_for_validators: matches.is_present("skip-wait-for-validators"),
    };

    if let Err(e) = solana_validator::validator::run_validator(config) {
        eprintln!("Validator failed to start: {}", e);
        process::exit(1);
    }
}
