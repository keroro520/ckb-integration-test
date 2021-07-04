pub mod case;
pub mod testdata;

use clap::{value_t, ArgMatches};
use lazy_static::lazy_static;
use std::env::current_dir;
use std::path::PathBuf;
use std::sync::RwLock;

lazy_static! {
    pub static ref CKB2019: RwLock<PathBuf> = RwLock::new(PathBuf::new());
    pub static ref CKB2021: RwLock<PathBuf> = RwLock::new(PathBuf::new());
}

pub fn init_ckb_binaries(matches: &ArgMatches) {
    let ckb2019 = value_t!(matches, "ckb2019", PathBuf)
        .unwrap_or_else(|err| panic!("failed to parse --ckb2019, error: {}", err));
    let ckb2021 = value_t!(matches, "ckb2021", PathBuf)
        .unwrap_or_else(|err| panic!("failed to parse --ckb2021, error: {}", err));
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
        current_dir()
            .expect("getting current dir should be ok")
            .join(path)
    } else {
        path
    }
}
