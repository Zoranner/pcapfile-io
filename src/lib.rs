//! # PcapFile.IO - 高性能PCAP文件读写库
//!
//! 这是一个用Rust编写的高性能PCAP文件处理库，提供了完整的PCAP文件读写功能。
//! 采用标准四层架构设计，确保系统的可维护性和可扩展性。
//!
//! ## 架构设计
//!
//! ```
//! +-------------------------------------+
//! |    用户接口层 (API Layer)           |  <- 对外提供服务接口
//! +-------------------------------------+
//! |    业务逻辑层 (Business Layer)      |  <- 核心业务逻辑实现
//! +-------------------------------------+
//! |    数据访问层 (Data Layer)          |  <- 数据操作和格式处理
//! +-------------------------------------+
//! |    基础设施层 (Foundation)          |  <- 工具函数和通用组件
//! +-------------------------------------+
//! ```
//!
//! ## 特性
//!
//! - 🚀 **高性能**: 零拷贝操作和编译时优化
//! - 🔒 **内存安全**: Rust的内存安全保证
//! - 🧵 **线程安全**: 内置线程安全支持
//! - 📦 **易于使用**: 简洁的API设计
//! - 🔧 **可配置**: 灵活的配置选项
//! - 📊 **完整功能**: 支持所有PCAP格式特性
//! - 📋 **索引支持**: 高性能PIDX索引文件处理
//!
//! ## 快速开始
//!
//! ```rust
//! use pcap_io::{
//!     ReaderConfig,
//!     WriterConfig,
//!     DataPacket,
//!     PcapReader,
//!     PcapWriter,
//!     Result,
//! };
//!
//! fn main() -> Result<()> {
//!     // 创建写入器配置
//!     let writer_config = WriterConfig::default();
//!
//!     // 写入PCAP数据集
//!     let mut writer = PcapWriter::new_with_config("./data", "example_dataset", writer_config)?;
//!
//!     let data = b"Hello, World!".to_vec();
//!     let packet = DataPacket::from_datetime(
//!         std::time::SystemTime::now(),
//!         data,
//!     )?;
//!
//!     writer.write_packet(&packet)?;
//!     writer.finalize()?;
//!
//!     // 创建读取器配置
//!     let reader_config = ReaderConfig::default();
//!
//!     // 读取PCAP数据集
//!     let mut reader = PcapReader::new_with_config("./data", "example_dataset", reader_config)?;
//!
//!     while let Some(packet) = reader.read_packet()? {
//!         println!("读取数据包: {:?}", packet);
//!     }
//!
//!     Ok(())
//! }
//! ```

// 分层架构模块声明
pub mod api;
pub mod business;
pub mod data;
pub mod foundation;

// 重新导出核心类型和函数
pub use business::{
    CommonConfig, PacketIndexEntry, PcapFileIndex,
    PidxIndex, ReaderConfig, WriterConfig,
};
pub use data::{
    DataPacket, DataPacketHeader, DatasetInfo, FileInfo,
    PcapFileHeader,
};
pub use foundation::{PcapError, PcapResult};

// 基础设施层类型导出
pub use foundation::{constants, PcapErrorCode};

// 用户接口层导出（主要API）
// 索引功能通过 PcapReader.index() 和 PcapWriter.index() 访问
pub use api::{PcapReader, PcapWriter};

// 版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// 获取库版本信息
pub fn version_info() -> &'static str {
    VERSION
}

/// 获取库作者信息
pub fn authors_info() -> &'static str {
    AUTHORS
}

/// 获取库描述信息
pub fn description_info() -> &'static str {
    DESCRIPTION
}
