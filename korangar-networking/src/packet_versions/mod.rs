pub mod lengths_20220406;
pub mod version_20220406;

/// All supported packet versions.
#[derive(Debug, Clone, Copy)]
pub enum SupportedPacketVersion {
    _20220406,
}
