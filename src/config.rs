//! # Redis server configuration.
//!
//! Things like the directory and filename of the [`Database`].

use std::path::PathBuf;
use structopt::StructOpt;

const DEFAULT_DIR: &str = ".";
const DEFAULT_FILE: &str = "db.rdb";

/// Redis server configuration.
#[derive(Debug, Clone, StructOpt)]
pub struct Config {
    // Redis uses `.rdb` files for persistence.
    // There are two config values that determine where RDB files are stored:
    //
    /// The directory where RDB files are stored.
    #[structopt(long, default_value = DEFAULT_DIR, parse(from_os_str))]
    pub(crate) dir: PathBuf,
    /// The name of the RDB file.
    #[structopt(long, default_value = DEFAULT_FILE, parse(from_os_str))]
    pub(crate) dbfilename: PathBuf,
}
