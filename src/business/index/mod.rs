//! Index模块 - 高性能PCAP索引文件处理
//!
//! 提供PCAP文件的索引生成、读取和管理功能，支持快速时间戳查找和范围查询。

pub mod manager;
pub mod types;

// 重新导出主要类型 - 统一使用IndexManager
pub use manager::IndexManager;

// 重新导出数据结构
pub use types::{PacketIndexEntry, PcapFileIndex, PidxIndex};
