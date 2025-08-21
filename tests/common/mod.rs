//! 测试公共工具模块
//!
//! 提供所有测试文件共用的辅助函数和工具

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::SystemTime;

use pcapfile_io::{DataPacket, PcapResult};

/// 测试输出基础路径
#[allow(dead_code)]
pub const TEST_BASE_PATH: &str = "test_output";

/// 清理指定数据集目录
#[allow(dead_code)]
pub fn clean_dataset_directory<P: AsRef<Path>>(
    dataset_path: P,
) -> PcapResult<()> {
    let path = dataset_path.as_ref();
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(pcapfile_io::PcapError::Io)?;
    }
    fs::create_dir_all(path)
        .map_err(pcapfile_io::PcapError::Io)?;
    Ok(())
}

/// 设置测试环境，为每个测试创建唯一目录并返回路径
#[allow(dead_code)]
pub fn setup_test_environment(
    test_name: &str,
) -> PcapResult<std::path::PathBuf> {
    let base_path = Path::new(TEST_BASE_PATH);
    if !base_path.exists() {
        fs::create_dir_all(base_path)
            .map_err(pcapfile_io::PcapError::Io)?;
    }

    let dataset_path = base_path.join(test_name);
    if dataset_path.exists() {
        fs::remove_dir_all(&dataset_path)
            .map_err(pcapfile_io::PcapError::Io)?;
    }
    fs::create_dir_all(&dataset_path)
        .map_err(pcapfile_io::PcapError::Io)?;

    Ok(dataset_path)
}

/// 创建基础测试数据包
#[allow(dead_code)]
pub fn create_test_packet(
    sequence: u32,
    size: usize,
) -> PcapResult<DataPacket> {
    let mut data = vec![0u8; size];
    for (i, item) in data.iter_mut().enumerate().take(size)
    {
        *item = (i + sequence as usize) as u8;
    }
    let capture_time = SystemTime::now();
    Ok(DataPacket::from_datetime(capture_time, data)?)
}

/// 创建具有特定模式的测试数据包（用于数据一致性测试）
#[allow(dead_code)]
pub fn create_detailed_test_packet(
    sequence: usize,
    size: usize,
) -> PcapResult<DataPacket> {
    let mut data = vec![0u8; size];

    // 创建具有清晰模式的数据，以便检测损坏
    for (i, item) in data.iter_mut().enumerate().take(size)
    {
        *item = match i % 4 {
            0 => (sequence % 256) as u8,
            1 => ((sequence >> 8) % 256) as u8,
            2 => (i % 256) as u8,
            3 => ((i >> 8) % 256) as u8,
            _ => unreachable!(),
        };
    }

    let capture_time = SystemTime::now();
    Ok(DataPacket::from_datetime(capture_time, data)?)
}

/// 创建大规模测试数据包（用于性能测试）
#[allow(dead_code)]
pub fn create_large_test_packet(
    sequence: usize,
    size: usize,
) -> PcapResult<DataPacket> {
    let mut data = vec![0u8; size];

    // 填充测试数据模式 - 使用更复杂的模式以避免压缩
    for (i, item) in data.iter_mut().enumerate().take(size)
    {
        *item = ((sequence * 31 + i * 17) % 256) as u8;
    }

    let capture_time = SystemTime::now();
    Ok(DataPacket::from_datetime(capture_time, data)?)
}

/// 创建小数据集测试数据包
#[allow(dead_code)]
pub fn create_small_test_packet(
    sequence: usize,
    size: usize,
) -> PcapResult<DataPacket> {
    let mut data = vec![0u8; size];

    // 填充测试数据
    for (i, item) in data.iter_mut().enumerate().take(size)
    {
        *item = ((sequence + i) % 256) as u8;
    }

    let capture_time = SystemTime::now();
    Ok(DataPacket::from_datetime(capture_time, data)?)
}

/// 计算数据哈希
#[allow(dead_code)]
pub fn calculate_data_hash(data: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// 数据包详细信息结构
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct PacketDetails {
    pub index: usize,
    pub timestamp_ns: u64,
    pub packet_length: u32,
    pub checksum: u32,
    pub data_hash: String, // 数据内容的哈希值
    pub first_16_bytes: Vec<u8>,
    pub last_16_bytes: Vec<u8>,
}

/// 从数据包创建详细信息
#[allow(dead_code)]
pub fn create_packet_details(
    packet: &DataPacket,
    index: usize,
) -> PacketDetails {
    let data_hash = calculate_data_hash(&packet.data);
    let first_16_bytes =
        packet.data.iter().take(16).cloned().collect();
    let last_16_bytes = packet
        .data
        .iter()
        .rev()
        .take(16)
        .cloned()
        .collect();

    PacketDetails {
        index,
        timestamp_ns: packet.get_timestamp_ns(),
        packet_length: packet.packet_length() as u32,
        checksum: packet.checksum(),
        data_hash,
        first_16_bytes,
        last_16_bytes,
    }
}

/// 数据包信息结构，用于验证
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PacketInfo {
    pub index: usize,
    pub packet_length: u32,
    pub checksum: u32,
    pub first_bytes: Vec<u8>,
}

/// 从数据包创建基础信息
#[allow(dead_code)]
pub fn create_packet_info(
    packet: &DataPacket,
    index: usize,
) -> PacketInfo {
    PacketInfo {
        index,
        packet_length: packet.packet_length() as u32,
        checksum: packet.checksum(),
        first_bytes: packet
            .data
            .iter()
            .take(16)
            .cloned()
            .collect(),
    }
}
