use crate::foundation::types::PcapErrorCode;
use thiserror::Error;

/// PCAP操作错误
#[derive(Error, Debug)]
pub enum PcapError {
    #[error("文件未找到: {0}")]
    FileNotFound(String),

    #[error("目录不存在: {0}")]
    DirectoryNotFound(String),

    #[error("无效的文件格式: {0}")]
    InvalidFormat(String),

    #[error("文件头损坏: {0}")]
    CorruptedHeader(String),

    #[error("数据包损坏: {message}，位置 {position}")]
    CorruptedData { message: String, position: u64 },

    #[error(
        "校验和不匹配: 期望 {expected}, 实际 {actual}，位置 {position}"
    )]
    ChecksumMismatch {
        expected: String,
        actual: String,
        position: u64,
    },

    #[error("数据包大小无效: {message}，位置 {position}")]
    InvalidPacketSize { message: String, position: u64 },

    #[error("数据包长度超出文件剩余空间: 期望 {expected} 字节，剩余 {remaining} 字节，位置 {position}")]
    PacketSizeExceedsRemainingBytes {
        expected: u32,
        remaining: u64,
        position: u64,
    },

    #[error("时间戳解析错误: {message}，位置 {position}")]
    TimestampParseError { message: String, position: u64 },

    #[error("参数无效: {0}")]
    InvalidArgument(String),

    #[error("操作状态无效: {0}")]
    InvalidState(String),

    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    Serialization(String),

    #[error("未知错误: {0}")]
    Unknown(String),
}

impl PcapError {
    /// 获取错误代码
    pub fn error_code(&self) -> PcapErrorCode {
        match self {
            PcapError::FileNotFound(_) => {
                PcapErrorCode::FileNotFound
            }
            PcapError::DirectoryNotFound(_) => {
                PcapErrorCode::DirectoryNotFound
            }
            PcapError::InvalidFormat(_) => {
                PcapErrorCode::InvalidFormat
            }
            PcapError::CorruptedHeader(_) => {
                PcapErrorCode::CorruptedHeader
            }
            PcapError::CorruptedData { .. } => {
                PcapErrorCode::CorruptedData
            }
            PcapError::ChecksumMismatch { .. } => {
                PcapErrorCode::ChecksumMismatch
            }
            PcapError::InvalidPacketSize { .. } => {
                PcapErrorCode::InvalidPacketSize
            }
            PcapError::PacketSizeExceedsRemainingBytes { .. } => {
                PcapErrorCode::CorruptedData
            }
            PcapError::TimestampParseError { .. } => {
                PcapErrorCode::CorruptedData
            }
            PcapError::InvalidArgument(_) => {
                PcapErrorCode::InvalidArgument
            }
            PcapError::InvalidState(_) => {
                PcapErrorCode::InvalidState
            }
            PcapError::Io(_) => PcapErrorCode::Unknown,
            PcapError::Serialization(_) => {
                PcapErrorCode::InvalidFormat
            }
            PcapError::Unknown(_) => PcapErrorCode::Unknown,
        }
    }

    /// 获取详细错误信息
    pub fn detailed_message(&self) -> String {
        format!(
            "错误代码: {}, 错误信息: {}",
            self.error_code(),
            self
        )
    }
}

/// 结果类型别名
pub type PcapResult<T> = std::result::Result<T, PcapError>;

/// 从字符串错误转换为PcapError
impl From<String> for PcapError {
    fn from(err: String) -> Self {
        PcapError::Unknown(err)
    }
}

/// 从&str错误转换为PcapError
impl From<&str> for PcapError {
    fn from(err: &str) -> Self {
        PcapError::Unknown(err.to_string())
    }
}

/// 从serde_json错误转换为PcapError
impl From<serde_json::Error> for PcapError {
    fn from(err: serde_json::Error) -> Self {
        PcapError::Serialization(err.to_string())
    }
}

/// 从base64错误转换为PcapError
impl From<base64::DecodeError> for PcapError {
    fn from(err: base64::DecodeError) -> Self {
        PcapError::InvalidFormat(format!(
            "Base64解码失败: {err}"
        ))
    }
}

/// 从std::string::FromUtf8Error错误转换为PcapError
impl From<std::string::FromUtf8Error> for PcapError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        PcapError::InvalidFormat(format!(
            "UTF8解码失败: {err}"
        ))
    }
}

/// 错误结果类型
#[derive(Debug, Clone)]
pub struct ErrorResult {
    pub success: bool,
    pub error_message: Option<String>,
    pub error_code: Option<PcapErrorCode>,
}

impl ErrorResult {
    /// 创建成功结果
    pub fn success() -> Self {
        Self {
            success: true,
            error_message: None,
            error_code: None,
        }
    }

    /// 创建失败结果
    pub fn failure(
        error_message: String,
        error_code: Option<PcapErrorCode>,
    ) -> Self {
        Self {
            success: false,
            error_message: Some(error_message),
            error_code,
        }
    }

    /// 从PcapError创建失败结果
    pub fn from_error(error: PcapError) -> Self {
        Self::failure(
            error.to_string(),
            Some(error.error_code()),
        )
    }
}

impl std::fmt::Display for ErrorResult {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        if self.success {
            write!(f, "操作成功")
        } else {
            write!(
                f,
                "操作失败: {} (错误代码: {:?})",
                self.error_message
                    .as_deref()
                    .unwrap_or("未知错误"),
                self.error_code
            )
        }
    }
}
