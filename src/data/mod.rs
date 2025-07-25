//! 数据访问层 - 数据模型定义和底层文件IO操作
//!
//! 负责底层文件读写操作、数据序列化/反序列化和格式解析生成。

pub mod file_reader;
pub mod file_writer;
pub mod formats;
pub mod models;

// 重新导出核心数据结构
pub use file_reader::PcapFileReader;
pub use file_writer::PcapFileWriter;
pub use formats::PcapFormatProcessor;
pub use models::{
    DataPacket, DataPacketHeader, DatasetInfo, FileInfo, PcapFileHeader,
};
