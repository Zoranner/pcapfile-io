//! 用户接口层 - 对外提供的统一API接口
//!
//! 提供用户友好的API接口，隐藏内部实现复杂性，实现资源的自动化管理。

pub mod reader;
pub mod writer;

// 重新导出用户API
pub use reader::PcapReader;
pub use writer::PcapWriter;
