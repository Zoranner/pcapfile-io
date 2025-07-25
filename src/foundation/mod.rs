//! 基础设施层 - 核心trait定义、错误处理和通用工具
//!
//! 提供整个库的基础设施支持，包括错误类型定义、核心trait接口和通用工具函数。

pub mod error;
pub mod types;
pub mod utils;

// 重新导出核心类型
pub use error::{PcapError, PcapResult};
pub use types::{constants, PcapErrorCode};
pub use utils::{
    binary_converter, calculate_crc32, ByteArrayExtensions,
    DateTimeExtensions,
};
