mod bench;
mod prepare;
mod stat;
mod watcher;

#[cfg(test)]
mod tests;

use crate::bench::{LiveCellProducer, TransactionProducer};
use crate::prepare::{collect, dispatch, generate_privkeys};
use crate::watcher::Watcher;
use ckb_crypto::secp::Privkey;
use ckb_testkit::{Node, User};
use ckb_types::{core::BlockNumber, packed::Byte32, prelude::*, H256};
use clap::{value_t_or_exit, values_t_or_exit, App, Arg, ArgMatches, SubCommand};
use crossbeam_channel::bounded;
use std::env;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};
use url::Url;

#[macro_export]
macro_rules! prompt_and_exit {
    ($($arg:tt)*) => ({
        eprintln!($($arg)*);
        ckb_testkit::error!($($arg)*);
        ::std::process::exit(1);
    })
}

fn main() {
    let _ = init_logger();
    entrypoint(clap_app().get_matches());
}

// TODO naming millis, ms
pub fn entrypoint(clap_arg_match: ArgMatches<'static>) {
    match clap_arg_match.subcommand() {
        // FIXME Currently, when specified `--n_blocks 10`, as we mine on different nodes,
        // the chain may not grow up 10 height.
        ("mine", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let n_blocks = value_t_or_exit!(arguments, "n_blocks", u64);
            let block_time_millis = value_t_or_exit!(arguments, "block_time_millis", u64);
            let miners = rpc_urls
                .iter()
                .map(|url| Node::init_from_url(url.as_str(), Default::default()))
                .collect::<Vec<_>>();
            let mut mined_n_blocks = 0;
            loop {
                for miner in miners.iter() {
                    miner.mine(1);
                    if n_blocks != 0 {
                        mined_n_blocks += 1;
                    }
                    if n_blocks != 0 && mined_n_blocks >= n_blocks {
                        return;
                    }
                    if block_time_millis != 0 {
                        sleep(Duration::from_millis(block_time_millis));
                    }
                }
            }
        }
        ("dispatch", Some(arguments)) => {
            let working_dir = value_t_or_exit!(arguments, "working_dir", PathBuf);
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let nodes = rpc_urls
                .iter()
                .map(|url| {
                    let port = url.port().unwrap();
                    let node_working_dir = working_dir.join(&port.to_string());
                    ::std::fs::create_dir_all(&node_working_dir).unwrap_or_else(|err| {
                        panic!(
                            "failed to create dir \"{}\", error: {}",
                            node_working_dir.display(),
                            err
                        )
                    });

                    Node::init_from_url(url.as_str(), node_working_dir)
                })
                .collect::<Vec<_>>();
            let n_borrowers = value_t_or_exit!(arguments, "n_borrowers", usize);
            let borrow_capacity = value_t_or_exit!(arguments, "borrow_capacity", u64);
            let lender_raw_privkey = env::var("CKB_BENCH_LENDER_PRIVKEY").unwrap_or_else(|err| {
                prompt_and_exit!("cannot find \"CKB_BENCH_LENDER_PRIVKEY\" from environment variables, error: {}", err)
            });
            let lender = {
                let lender_privkey = Privkey::from_str(&lender_raw_privkey).unwrap_or_else(|err| {
                    prompt_and_exit!(
                        "failed to parse CKB_BENCH_LENDER_PRIVKEY to Privkey, error: {}",
                        err
                    )
                });
                User::new(nodes[0].get_block_by_number(0), Some(lender_privkey))
            };
            let borrowers = {
                let lender_byte32_privkey =
                    Byte32::from_slice(H256::from_str(&lender_raw_privkey).unwrap().as_bytes())
                        .unwrap_or_else(|err| {
                            prompt_and_exit!(
                                "failed to parse CKB_BENCH_LENDER_PRIVKEY to Byte32, error: {}",
                                err
                            )
                        });
                let privkeys = generate_privkeys(lender_byte32_privkey, n_borrowers);
                privkeys
                    .into_iter()
                    .map(|privkey| User::new(nodes[0].get_block_by_number(0), Some(privkey)))
                    .collect::<Vec<_>>()
            };
            dispatch(&nodes, &lender, &borrowers, borrow_capacity);
        }
        ("collect", Some(arguments)) => {
            let working_dir = value_t_or_exit!(arguments, "working_dir", PathBuf);
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let nodes = rpc_urls
                .iter()
                .map(|url| {
                    let port = url.port().unwrap();
                    let node_working_dir = working_dir.join(&port.to_string());
                    ::std::fs::create_dir_all(&node_working_dir).unwrap_or_else(|err| {
                        panic!(
                            "failed to create dir \"{}\", error: {}",
                            node_working_dir.display(),
                            err
                        )
                    });
                    Node::init_from_url(url.as_str(), node_working_dir)
                })
                .collect::<Vec<_>>();
            let n_borrowers = value_t_or_exit!(arguments, "n_borrowers", usize);
            let lender_raw_privkey = env::var("CKB_BENCH_LENDER_PRIVKEY").unwrap_or_else(|err| {
                prompt_and_exit!("cannot find \"CKB_BENCH_LENDER_PRIVKEY\" from environment variables, error: {}", err)
            });
            let lender = {
                let lender_privkey = Privkey::from_str(&lender_raw_privkey).unwrap_or_else(|err| {
                    prompt_and_exit!(
                        "failed to parse CKB_BENCH_LENDER_PRIVKEY to Privkey, error: {}",
                        err
                    )
                });
                User::new(nodes[0].get_block_by_number(0), Some(lender_privkey))
            };
            let borrowers = {
                let lender_byte32_privkey =
                    Byte32::from_slice(H256::from_str(&lender_raw_privkey).unwrap().as_bytes())
                        .unwrap_or_else(|err| {
                            prompt_and_exit!(
                                "failed to parse CKB_BENCH_LENDER_PRIVKEY to Byte32, error: {}",
                                err
                            )
                        });
                let privkeys = generate_privkeys(lender_byte32_privkey, n_borrowers);
                privkeys
                    .into_iter()
                    .map(|privkey| User::new(nodes[0].get_block_by_number(0), Some(privkey)))
                    .collect::<Vec<_>>()
            };
            collect(&nodes, &lender, &borrowers);
        }
        ("bench", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let working_dir = value_t_or_exit!(arguments, "working_dir", PathBuf);
            let nodes = rpc_urls
                .iter()
                .map(|url| {
                    let port = url.port().unwrap();
                    let node_working_dir = working_dir.join(&port.to_string());
                    ::std::fs::create_dir_all(&node_working_dir).unwrap_or_else(|err| {
                        panic!(
                            "failed to create dir \"{}\", error: {}",
                            node_working_dir.display(),
                            err
                        )
                    });
                    Node::init_from_url(url.as_str(), node_working_dir)
                })
                .collect::<Vec<_>>();
            let n_borrowers = value_t_or_exit!(arguments, "n_borrowers", usize);
            let n_outputs = value_t_or_exit!(arguments, "n_outputs", usize);
            let delay_ms = value_t_or_exit!(arguments, "delay_ms", u64);
            let lender_raw_privkey = env::var("CKB_BENCH_LENDER_PRIVKEY").unwrap_or_else(|err| {
                prompt_and_exit!("cannot find \"CKB_BENCH_LENDER_PRIVKEY\" from environment variables, error: {}", err)
            });
            let borrowers = {
                let lender_byte32_privkey =
                    Byte32::from_slice(H256::from_str(&lender_raw_privkey).unwrap().as_bytes())
                        .unwrap_or_else(|err| {
                            prompt_and_exit!(
                                "failed to parse CKB_BENCH_LENDER_PRIVKEY to Byte32, error: {}",
                                err
                            )
                        });
                let privkeys = generate_privkeys(lender_byte32_privkey, n_borrowers);
                privkeys
                    .into_iter()
                    .map(|privkey| User::new(nodes[0].get_block_by_number(0), Some(privkey)))
                    .collect::<Vec<_>>()
            };
            let (live_cell_sender, live_cell_receiver) = bounded(1000);
            let (transaction_sender, transaction_receiver) = bounded(1000);

            let live_cell_producer = LiveCellProducer::new(borrowers.clone(), nodes.clone());
            spawn(move || {
                live_cell_producer.run(live_cell_sender);
            });

            let transaction_producer = TransactionProducer::new(
                borrowers.clone(),
                vec![borrowers[0].single_secp256k1_cell_dep()],
                n_outputs,
            );
            spawn(move || {
                transaction_producer.run(live_cell_receiver, transaction_sender);
            });

            let watcher = Watcher::new(nodes.clone().into());
            while !watcher.is_zero_load() {
                sleep(Duration::from_secs(10));
                ckb_testkit::info!(
                    "[Watcher] is waiting the node become zero-load, fixed_tip_number: {}",
                    watcher.get_fixed_header().number()
                );
            }

            let zero_load_number = watcher.get_fixed_header().number();
            let mut i = 0;
            let mut start_time = Instant::now();
            let t_delay = Duration::from_millis(delay_ms);
            while let Ok(tx) = transaction_receiver.recv() {
                if t_delay.as_millis() != 0 {
                    sleep(t_delay);
                }

                i = (i + 1) % nodes.len();
                let result = nodes[i]
                    .rpc_client()
                    .send_transaction_result(tx.data().into());
                // TODO if error is TxPoolFull, then retry until success
                if let Err(err) = result {
                    ckb_testkit::error!("failed to send {:#x}, error: {:?}", tx.hash(), err);
                }

                if start_time.elapsed() > Duration::from_secs(60) {
                    start_time = Instant::now();
                    if watcher.is_steady_load(zero_load_number) {
                        println!(
                            "is_stead_load({}, {}) returns true",
                            zero_load_number,
                            watcher.get_fixed_header().number()
                        );
                        ckb_testkit::info!(
                            "is_stead_load({}, {}) returns true",
                            zero_load_number,
                            watcher.get_fixed_header().number()
                        );
                        break;
                    }
                }
            }

            let metrics = watcher.calc_recent_metrics(zero_load_number);
            println!("metrics: {:?}", metrics);
            ckb_testkit::info!("metrics: {:?}", metrics);
        }
        ("stat", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let from_number = value_t_or_exit!(arguments, "from_number", BlockNumber);
            let to_number = value_t_or_exit!(arguments, "to_number", BlockNumber);
            let stat_time_ms = value_t_or_exit!(arguments, "stat_time_ms", u64);
            let stat_time = Duration::from_millis(stat_time_ms);
            let node = Node::init_from_url(rpc_urls[0].as_str(), Default::default());
            let metrics = stat::stat(&node, from_number, to_number, stat_time);
            println!("metrics: {:?}", metrics);
            ckb_testkit::info!("metrics: {:?}", metrics);
        }
        _ => {
            eprintln!("wrong usage");
            exit(1);
        }
    }
}

fn clap_app() -> App<'static, 'static> {
    include_str!("../Cargo.toml");
    App::new("ckb-bench")
        .subcommand(
            SubCommand::with_name("mine")
                .about("Mine specified number of blocks")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n_blocks")
                        .short("b")
                        .long("n_blocks")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("number of blocks to mine, default is infinite(0)")
                        .default_value("0")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("block_time_millis")
                        .long("block_time_millis")
                        .value_name("TIME")
                        .takes_value(true)
                        .help("block time, default is 0")
                        .default_value("0")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                ),
        )
        .subcommand(
            SubCommand::with_name("bench")
                .about("bench the target ckb nodes")
                .arg(
                    Arg::with_name("working_dir")
                        .long("working_dir")
                        .required(true)
                        .takes_value(true)
                        .value_name("PATH")
                        .default_value(".")
                        .help("path to working directory"),
                )
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n_borrowers")
                        .long("n_borrowers")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("number of borrowers")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n_outputs")
                        .long("n_outputs")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("count of outputs of a transaction")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("delay_ms")
                        .long("delay_ms")
                        .value_name("TIME")
                        .takes_value(true)
                        .help("delay of sending transactions in milliseconds")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                ),
        )
        .subcommand(
            SubCommand::with_name("dispatch")
                .about("dispatch lender's capacity to borrowers")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n_borrowers")
                        .long("n_borrowers")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("number of borrowers")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("borrow_capacity")
                        .long("borrow_capacity")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("how much capacity to borrow")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("working_dir")
                        .long("working_dir")
                        .required(true)
                        .takes_value(true)
                        .value_name("PATH")
                        .default_value(".")
                        .help("path to working directory"),
                ),
        )
        .subcommand(
            SubCommand::with_name("collect")
                .about("collect borrowers' capacity back to lender")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n_borrowers")
                        .long("n_borrowers")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("number of borrowers")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("working_dir")
                        .long("working_dir")
                        .required(true)
                        .takes_value(true)
                        .value_name("PATH")
                        .default_value(".")
                        .help("path to working directory"),
                ),
        )
        .subcommand(
            SubCommand::with_name("stat")
                .about("report chain stat")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("from_number")
                        .long("from_number")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("block number to stat from")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("to_number")
                        .long("to_number")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("block number to stat to")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("stat_time_ms")
                        .long("stat_time_ms")
                        .value_name("TIME")
                        .takes_value(true)
                        .help("duration to stat")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                ),
        )
}

fn init_logger() -> ckb_logger_service::LoggerInitGuard {
    let filter = match env::var("RUST_LOG") {
        Ok(filter) if filter.is_empty() => Some("info".to_string()),
        Ok(filter) => Some(filter.to_string()),
        Err(_) => Some("info".to_string()),
    };
    let config = ckb_logger_config::Config {
        filter,
        log_to_file: false,
        log_to_stdout: true,
        ..Default::default()
    };
    ckb_logger_service::init(None, config)
        .unwrap_or_else(|err| panic!("failed to init the logger service, error: {}", err))
}
