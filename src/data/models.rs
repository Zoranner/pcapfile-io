use crate::foundation::types::constants;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// PCAP文件头结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcapFileHeader {
    /// 魔术数，固定值 0xD4C3B2A1
    pub magic_number: u32,
    /// 主版本号，固定值 0x0002
    pub major_version: u16,
    /// 次版本号，固定值 0x0004
    pub minor_version: u16,
    /// 时区偏移量（秒）
    pub timezone_offset: i32,
    /// 时间戳精度（纳秒）
    pub timestamp_accuracy: u32,
}

impl PcapFileHeader {
    /// 头部大小（字节）
    pub const HEADER_SIZE: usize = 16; // 4 + 2 + 2 + 4 + 4

    /// 默认时间戳精度（纳秒）
    pub const DEFAULT_TIMESTAMP_ACCURACY: u32 = 1;

    /// 创建新的PCAP文件头
    pub fn new(timezone_offset: i32) -> Self {
        Self {
            magic_number: constants::PCAP_MAGIC_NUMBER,
            major_version: constants::MAJOR_VERSION,
            minor_version: constants::MINOR_VERSION,
            timezone_offset,
            timestamp_accuracy: Self::DEFAULT_TIMESTAMP_ACCURACY,
        }
    }

    /// 从字节数组创建文件头
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < Self::HEADER_SIZE {
            return Err("字节数组长度不足".to_string());
        }

        let magic_number = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let major_version = u16::from_le_bytes([bytes[4], bytes[5]]);
        let minor_version = u16::from_le_bytes([bytes[6], bytes[7]]);
        let timezone_offset = i32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let timestamp_accuracy = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);

        Ok(Self {
            magic_number,
            major_version,
            minor_version,
            timezone_offset,
            timestamp_accuracy,
        })
    }

    /// 转换为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::HEADER_SIZE);
        bytes.extend_from_slice(&self.magic_number.to_le_bytes());
        bytes.extend_from_slice(&self.major_version.to_le_bytes());
        bytes.extend_from_slice(&self.minor_version.to_le_bytes());
        bytes.extend_from_slice(&self.timezone_offset.to_le_bytes());
        bytes.extend_from_slice(&self.timestamp_accuracy.to_le_bytes());
        bytes
    }

    /// 验证文件头是否有效
    pub fn is_valid(&self) -> bool {
        self.magic_number == constants::PCAP_MAGIC_NUMBER
            && self.major_version == constants::MAJOR_VERSION
            && self.minor_version == constants::MINOR_VERSION
    }
}

/// 数据包头部结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPacketHeader {
    /// 时间戳（秒）
    pub timestamp_seconds: u32,
    /// 时间戳（纳秒）
    pub timestamp_nanoseconds: u32,
    /// 数据包长度
    pub packet_length: u32,
    /// 校验和
    pub checksum: u32,
}

impl DataPacketHeader {
    /// 头部大小（字节）
    pub const HEADER_SIZE: usize = 16; // 4 + 4 + 4 + 4

    /// 创建新的数据包头部
    pub fn new(
        timestamp_seconds: u32,
        timestamp_nanoseconds: u32,
        packet_length: u32,
        checksum: u32,
    ) -> Result<Self, String> {
        if !DataPacket::is_valid_size(packet_length as usize) {
            return Err(format!("无效的数据包长度: {}", packet_length));
        }

        Ok(Self {
            timestamp_seconds,
            timestamp_nanoseconds,
            packet_length,
            checksum,
        })
    }

    /// 从DateTime创建数据包头部
    pub fn from_datetime(
        capture_time: SystemTime,
        packet_length: u32,
        checksum: u32,
    ) -> Result<Self, String> {
        let duration = capture_time
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "时间戳无效")?;

        let timestamp_seconds = duration.as_secs() as u32;
        let timestamp_nanoseconds = duration.subsec_nanos();

        Self::new(
            timestamp_seconds,
            timestamp_nanoseconds,
            packet_length,
            checksum,
        )
    }

    /// 从数据包数据创建头部
    pub fn from_packet_data(capture_time: SystemTime, packet_data: &[u8]) -> Result<Self, String> {
        let checksum = crate::foundation::utils::calculate_crc32(packet_data);
        let packet_length = packet_data.len() as u32;

        Self::from_datetime(capture_time, packet_length, checksum)
    }

    /// 从字节数组创建头部
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < Self::HEADER_SIZE {
            return Err("字节数组长度不足".to_string());
        }

        let timestamp_seconds = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let timestamp_nanoseconds = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let packet_length = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let checksum = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);

        Self::new(
            timestamp_seconds,
            timestamp_nanoseconds,
            packet_length,
            checksum,
        )
    }

    /// 转换为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::HEADER_SIZE);
        bytes.extend_from_slice(&self.timestamp_seconds.to_le_bytes());
        bytes.extend_from_slice(&self.timestamp_nanoseconds.to_le_bytes());
        bytes.extend_from_slice(&self.packet_length.to_le_bytes());
        bytes.extend_from_slice(&self.checksum.to_le_bytes());
        bytes
    }

    /// 获取捕获时间
    pub fn capture_time(&self) -> SystemTime {
        UNIX_EPOCH
            + std::time::Duration::new(self.timestamp_seconds as u64, self.timestamp_nanoseconds)
    }
}

/// 数据包结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPacket {
    /// 数据包头部
    pub header: DataPacketHeader,
    /// 数据包内容
    pub data: Vec<u8>,
}

impl DataPacket {
    /// 创建新的数据包
    pub fn new(header: DataPacketHeader, data: Vec<u8>) -> Result<Self, String> {
        if data.len() != header.packet_length as usize {
            return Err("数据长度与头部长度不匹配".to_string());
        }

        Ok(Self { header, data })
    }

    /// 从DateTime和数据创建数据包
    pub fn from_datetime(capture_time: SystemTime, data: Vec<u8>) -> Result<Self, String> {
        let header = DataPacketHeader::from_packet_data(capture_time, &data)?;
        Self::new(header, data)
    }

    /// 从时间戳和数据创建数据包
    pub fn from_timestamp(
        timestamp_seconds: u32,
        timestamp_nanoseconds: u32,
        data: Vec<u8>,
    ) -> Result<Self, String> {
        let checksum = crate::foundation::utils::calculate_crc32(&data);
        let packet_length = data.len() as u32;

        let header = DataPacketHeader::new(
            timestamp_seconds,
            timestamp_nanoseconds,
            packet_length,
            checksum,
        )?;

        Self::new(header, data)
    }

    /// 获取捕获时间
    pub fn capture_time(&self) -> SystemTime {
        self.header.capture_time()
    }

    /// 获取数据包长度
    pub fn packet_length(&self) -> usize {
        self.data.len()
    }

    /// 获取总大小（头部 + 数据）
    pub fn total_size(&self) -> usize {
        DataPacketHeader::HEADER_SIZE + self.packet_length()
    }

    /// 获取校验和
    pub fn checksum(&self) -> u32 {
        self.header.checksum
    }

    /// 获取时间戳（纳秒）
    pub fn get_timestamp_ns(&self) -> u64 {
        let duration = self
            .capture_time()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        duration.as_secs() * 1_000_000_000 + duration.subsec_nanos() as u64
    }

    /// 验证数据包大小是否有效
    pub fn is_valid_size(size: usize) -> bool {
        size > 0 && size <= constants::MAX_PACKET_SIZE
    }

    /// 验证数据包是否有效
    pub fn is_valid(&self) -> bool {
        if !Self::is_valid_size(self.packet_length()) {
            return false;
        }

        let calculated_checksum = crate::foundation::utils::calculate_crc32(&self.data);
        calculated_checksum == self.header.checksum
    }

    /// 转换为字节数组（头部 + 数据）
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.total_size());
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.data);
        bytes
    }
}

impl std::fmt::Display for DataPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DataPacket {{ timestamp: {:?}, length: {}, checksum: 0x{:08X} }}",
            self.capture_time(),
            self.packet_length(),
            self.checksum()
        )
    }
}

// PacketIndexEntry、PcapFileIndex、PidxIndex、PidxStats 及其 impl 移动到 src/index/types.rs

/// 数据集信息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetInfo {
    /// 数据集名称
    pub name: String,
    /// 数据集路径
    pub path: std::path::PathBuf,
    /// 包含的文件数量
    pub file_count: usize,
    /// 数据包总数
    pub total_packets: u64,
    /// 数据集总大小（字节）
    pub total_size: u64,
    /// 开始时间戳（纳秒）
    pub start_timestamp: Option<u64>,
    /// 结束时间戳（纳秒）
    pub end_timestamp: Option<u64>,
    /// 创建时间
    pub created_time: String,
    /// 最后修改时间
    pub modified_time: String,
    /// 是否包含索引文件
    pub has_index: bool,
}

impl DatasetInfo {
    /// 创建新的数据集信息
    pub fn new<P: AsRef<std::path::Path>>(name: String, path: P) -> Self {
        use chrono::Utc;

        Self {
            name,
            path: path.as_ref().to_path_buf(),
            file_count: 0,
            total_packets: 0,
            total_size: 0,
            start_timestamp: None,
            end_timestamp: None,
            created_time: Utc::now().to_rfc3339(),
            modified_time: Utc::now().to_rfc3339(),
            has_index: false,
        }
    }

    /// 获取时间范围
    pub fn time_range(&self) -> Option<(u64, u64)> {
        match (self.start_timestamp, self.end_timestamp) {
            (Some(start), Some(end)) => Some((start, end)),
            _ => None,
        }
    }

    /// 获取总时长（纳秒）
    pub fn total_duration_ns(&self) -> u64 {
        match self.time_range() {
            Some((start, end)) => end.saturating_sub(start),
            None => 0,
        }
    }

    /// 获取总时长（秒）
    pub fn total_duration_seconds(&self) -> f64 {
        self.total_duration_ns() as f64 / 1_000_000_000.0
    }

    /// 获取平均数据包速率（包/秒）
    pub fn average_packet_rate(&self) -> f64 {
        let duration = self.total_duration_seconds();
        if duration > 0.0 {
            self.total_packets as f64 / duration
        } else {
            0.0
        }
    }
}

/// 文件信息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// 文件名
    pub file_name: String,
    /// 文件路径
    pub file_path: std::path::PathBuf,
    /// 文件大小（字节）
    pub file_size: u64,
    /// 数据包数量
    pub packet_count: u64,
    /// 开始时间戳（纳秒）
    pub start_timestamp: Option<u64>,
    /// 结束时间戳（纳秒）
    pub end_timestamp: Option<u64>,
    /// 文件哈希值
    pub file_hash: Option<String>,
    /// 创建时间
    pub created_time: String,
    /// 最后修改时间
    pub modified_time: String,
    /// 是否有效
    pub is_valid: bool,
}

impl FileInfo {
    /// 创建新的文件信息
    pub fn new<P: AsRef<std::path::Path>>(file_path: P) -> Self {
        use chrono::Utc;

        let path = file_path.as_ref();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string();

        Self {
            file_name,
            file_path: path.to_path_buf(),
            file_size: 0,
            packet_count: 0,
            start_timestamp: None,
            end_timestamp: None,
            file_hash: None,
            created_time: Utc::now().to_rfc3339(),
            modified_time: Utc::now().to_rfc3339(),
            is_valid: false,
        }
    }

    /// 从文件系统获取基本信息
    pub fn from_file<P: AsRef<std::path::Path>>(file_path: P) -> Result<Self, std::io::Error> {
        use chrono::{DateTime, Utc};

        let path = file_path.as_ref();
        let metadata = std::fs::metadata(path)?;

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string();

        let created_time = metadata
            .created()
            .map(|time| DateTime::<Utc>::from(time).to_rfc3339())
            .unwrap_or_else(|_| Utc::now().to_rfc3339());

        let modified_time = metadata
            .modified()
            .map(|time| DateTime::<Utc>::from(time).to_rfc3339())
            .unwrap_or_else(|_| Utc::now().to_rfc3339());

        Ok(Self {
            file_name,
            file_path: path.to_path_buf(),
            file_size: metadata.len(),
            packet_count: 0,
            start_timestamp: None,
            end_timestamp: None,
            file_hash: None,
            created_time,
            modified_time,
            is_valid: path.exists() && metadata.is_file(),
        })
    }

    /// 获取时间范围
    pub fn time_range(&self) -> Option<(u64, u64)> {
        match (self.start_timestamp, self.end_timestamp) {
            (Some(start), Some(end)) => Some((start, end)),
            _ => None,
        }
    }

    /// 获取文件时长（纳秒）
    pub fn duration_ns(&self) -> u64 {
        match self.time_range() {
            Some((start, end)) => end.saturating_sub(start),
            None => 0,
        }
    }

    /// 获取文件时长（秒）
    pub fn duration_seconds(&self) -> f64 {
        self.duration_ns() as f64 / 1_000_000_000.0
    }

    /// 计算并设置文件哈希值
    pub fn calculate_hash(&mut self) -> Result<(), std::io::Error> {
        use sha2::{Digest, Sha256};
        use std::io::Read;

        let mut file = std::fs::File::open(&self.file_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        self.file_hash = Some(format!("{:x}", hasher.finalize()));
        Ok(())
    }
}
