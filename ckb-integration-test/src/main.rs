pub mod case;
pub mod testdata;

use clap::{value_t_or_exit, App, Arg, ArgMatches, SubCommand};
use lazy_static::lazy_static;
use std::env;
use std::path::PathBuf;
use std::process::exit;
use std::sync::RwLock;

lazy_static! {
    pub static ref CKB2019: RwLock<PathBuf> = RwLock::new(PathBuf::new());
    pub static ref CKB2021: RwLock<PathBuf> = RwLock::new(PathBuf::new());
}

fn main() {
    env::set_var("RUST_BACKTRACE", "full");
    let matches = clap_app().get_matches();
    let _logger = init_logger(&matches);

    match matches.subcommand() {
        ("run", Some(arg_matches)) => {
            crate::init_ckb_binaries(&arg_matches);
            let cases = crate::case::all_cases();
            for case in cases {
                crate::case::run_case(case);
            }
        }
        ("generate-testdata", Some(arg_matches)) => {
            crate::init_ckb_binaries(&arg_matches);
            let testdatas = crate::testdata::all_testdata_generators();
            for testdata in testdatas {
                testdata.generate();
            }
        }
        _ => {
            println!("invalid usage");
            exit(1);
        }
    }
}

fn clap_app() -> App<'static, 'static> {
    App::new("ckb-integration-test")
        .subcommand(
            SubCommand::with_name("run")
                .about("Run test cases")
                .arg(
                    Arg::with_name("ckb2019")
                        .long("ckb2019")
                        .takes_value(true)
                        .value_name("PATH")
                        .help("Path to ckb2019 executable"),
                )
                .arg(
                    Arg::with_name("ckb2021")
                        .long("ckb2021")
                        .takes_value(true)
                        .value_name("PATH")
                        .help("Path to ckb2021 executable"),
                ),
        )
        .subcommand(
            SubCommand::with_name("generate-testdata")
                .about("Run testdata generators")
                .arg(
                    Arg::with_name("ckb2019")
                        .long("ckb2019")
                        .takes_value(true)
                        .value_name("PATH")
                        .help("Path to ckb v1 executable"),
                )
                .arg(
                    Arg::with_name("ckb2021")
                        .long("ckb2021")
                        .takes_value(true)
                        .value_name("PATH")
                        .help("Path to ckb v2 executable"),
                ),
        )
}

fn init_logger(_clap_matches: &ArgMatches) -> ckb_logger_service::LoggerInitGuard {
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

fn init_ckb_binaries(matches: &ArgMatches) {
    let ckb2019 = value_t_or_exit!(matches, "ckb2019", PathBuf);
    let ckb2021 = value_t_or_exit!(matches, "ckb2021", PathBuf);
    if !ckb2019.exists() || !ckb2019.is_file() {
        panic!("--ckb2019 points to non-executable")
    }
    if !ckb2021.exists() || !ckb2021.is_file() {
        panic!("--ckb2021 points to non-executable")
    }
    *CKB2019.write().unwrap() = absolutize(ckb2019);
    *CKB2021.write().unwrap() = absolutize(ckb2021);
}

fn absolutize(path: PathBuf) -> PathBuf {
    if path.is_relative() {
        env::current_dir()
            .expect("getting current dir should be ok")
            .join(path)
    } else {
        path
    }
}