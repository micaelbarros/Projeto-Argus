//! Infrastructure adapters implementing `argus-core` hexagonal ports.

pub mod exporters;
pub mod filesystem;
pub mod git;
pub mod network;
pub mod persistence;

pub use exporters::DefaultReportExporter;
pub use filesystem::LocalFileSystemAdapter;
pub use git::Git2Adapter;
pub use network::SystemNetworkAdapter;
pub use persistence::SQLiteMetricsAdapter;
