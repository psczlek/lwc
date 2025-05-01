use clap::Parser;

#[derive(Parser)]
#[command(name = "lwc", version = "0.0.1", about = "A wc clone", long_about = None)]
pub struct Flag {
    /// Target files
    #[arg(required = true)]
    pub entries: Vec<String>,

    /// Traverse directories recursively
    #[arg(
        short = 'r',
        long = "recursive",
        required = false,
        default_value = "false"
    )]
    pub rflag: bool,

    /// Display stats in one line
    #[arg(
        short = 'o',
        long = "oneline",
        required = false,
        default_value = "false"
    )]
    pub oflag: bool,

    /// Count elements in a directory
    #[arg(short = 'f', required = false, default_value = "false")]
    pub fflag: bool,

    /// Print paths as absolute paths
    #[arg(
        short = 'a',
        long = "absolute-paths",
        required = false,
        default_value = "false"
    )]
    pub aflag: bool,
}
