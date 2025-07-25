//! 业务逻辑层 - 配置管理、缓存策略和业务流程编排
//!
//! 实现核心业务规则和算法，包括配置管理、索引系统和性能优化策略。

pub mod cache;
pub mod config;
pub mod index;
pub mod processor;

// 重新导出核心配置和索引类型
pub use cache::{CacheStats, FileInfoCache};
pub use config::{CommonConfig, ReaderConfig, WriterConfig};
pub use index::{PacketIndexEntry, PcapFileIndex, PidxIndex};
pub use processor::{PacketProcessor, ProcessedPacket, ProcessorStatistics, ValidationResult};

// IndexManager作为内部实现细节，不对外暴露
// 用户应该通过 PcapReader.index() 或 PcapWriter.index() 来访问索引功能
