//! 公共类型和常量定义
//!
//! 定义整个库使用的通用类型和常量，为所有层提供基础数据类型支持。

/// PCAP格式常量定义
pub mod constants {
    /// PCAP文件标识，固定值 0xD4C3B2A1
    pub const PCAP_MAGIC_NUMBER: u32 = 0xD4C3B2A1;

    /// PROJ文件标识 ("PROJ")
    pub const PROJ_MAGIC_NUMBER: u32 = 0xA1B2C3D4;

    /// 主版本号，固定值 0x0002
    pub const MAJOR_VERSION: u16 = 2;

    /// 次版本号，固定值 0x0004，表示支持纳秒级时间量
    pub const MINOR_VERSION: u16 = 4;

    /// 每个PCAP文件最大数据包数量
    pub const DEFAULT_MAX_PACKETS_PER_FILE: usize = 500;

    /// 最大缓冲区大小(字节)
    pub const MAX_BUFFER_SIZE: usize = 50 * 1024 * 1024; // 50MB

    /// 默认文件命名格式
    pub const DEFAULT_FILE_NAME_FORMAT: &str =
        "yyMMdd_HHmmss_fffffff";

    /// 数据包最大大小(字节)
    pub const MAX_PACKET_SIZE: usize = 30 * 1024 * 1024; // 30MB
}

/// 错误代码枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcapErrorCode {
    /// 未知错误
    Unknown = 0,
    /// 文件未找到
    FileNotFound = 1001,
    /// 目录不存在
    DirectoryNotFound = 1002,
    /// 权限不足
    InsufficientPermissions = 1003,
    /// 磁盘空间不足
    DiskSpaceFull = 1004,
    /// 无效的文件格式
    InvalidFormat = 2001,
    /// 文件头损坏
    CorruptedHeader = 2002,
    /// 数据包损坏
    CorruptedData = 2003,
    /// 校验和不匹配
    ChecksumMismatch = 2004,
    /// 数据包大小无效
    InvalidPacketSize = 3001,
    /// 参数无效
    InvalidArgument = 3002,
    /// 操作状态无效
    InvalidState = 3003,
    /// 缓冲区溢出
    BufferOverflow = 4001,
    /// 内存不足
    OutOfMemory = 4002,
}

impl std::fmt::Display for PcapErrorCode {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            PcapErrorCode::Unknown => write!(f, "未知错误"),
            PcapErrorCode::FileNotFound => {
                write!(f, "文件未找到")
            }
            PcapErrorCode::DirectoryNotFound => {
                write!(f, "目录不存在")
            }
            PcapErrorCode::InsufficientPermissions => {
                write!(f, "权限不足")
            }
            PcapErrorCode::DiskSpaceFull => {
                write!(f, "磁盘空间不足")
            }
            PcapErrorCode::InvalidFormat => {
                write!(f, "无效的文件格式")
            }
            PcapErrorCode::CorruptedHeader => {
                write!(f, "文件头损坏")
            }
            PcapErrorCode::CorruptedData => {
                write!(f, "数据包损坏")
            }
            PcapErrorCode::ChecksumMismatch => {
                write!(f, "校验和不匹配")
            }
            PcapErrorCode::InvalidPacketSize => {
                write!(f, "数据包大小无效")
            }
            PcapErrorCode::InvalidArgument => {
                write!(f, "参数无效")
            }
            PcapErrorCode::InvalidState => {
                write!(f, "操作状态无效")
            }
            PcapErrorCode::BufferOverflow => {
                write!(f, "缓冲区溢出")
            }
            PcapErrorCode::OutOfMemory => {
                write!(f, "内存不足")
            }
        }
    }
}
