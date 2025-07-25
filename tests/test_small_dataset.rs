//! 小数据集测试
//!
//! 测试基本的读写功能和索引生成

use pcap_io::{
    DataPacket, PcapReader, PcapWriter, ReaderConfig,
    PcapResult, WriterConfig,
};
use std::path::Path;
use std::time::SystemTime;
use tempfile::TempDir;

/// 数据包信息结构，用于验证
#[derive(Debug, Clone)]
struct PacketInfo {
    index: usize,
    packet_length: u32,
    checksum: u32,
    first_bytes: Vec<u8>,
}

/// 创建测试数据包
fn create_test_packet(
    sequence: usize,
    size: usize,
) -> PcapResult<DataPacket> {
    let mut data = vec![0u8; size];

    // 填充测试数据
    for i in 0..size {
        data[i] = ((sequence + i) % 256) as u8;
    }

    let capture_time = SystemTime::now();
    Ok(DataPacket::from_datetime(capture_time, data)?)
}

/// 写入小数据集
fn write_small_dataset(
    base_path: &Path,
    dataset_name: &str,
    packet_count: usize,
    packet_size: usize,
) -> PcapResult<Vec<PacketInfo>> {
    let config = WriterConfig::default();
    let mut writer = PcapWriter::new_with_config(
        base_path,
        dataset_name,
        config,
    )?;

    let mut written_packets = Vec::new();

    for i in 0..packet_count {
        let packet = create_test_packet(i, packet_size)?;
        written_packets.push(PacketInfo {
            index: i,
            packet_length: packet.packet_length() as u32,
            checksum: packet.checksum(),
            first_bytes: packet
                .data
                .iter()
                .take(16)
                .cloned()
                .collect(),
        });
        writer.write_packet(&packet)?;
    }

    writer.finalize()?;

    Ok(written_packets)
}

/// 读取测试数据包
fn read_test_packets(
    base_path: &Path,
    dataset_name: &str,
) -> PcapResult<Vec<PacketInfo>> {
    let mut reader = PcapReader::new_with_config(
        base_path,
        dataset_name,
        ReaderConfig::default(),
    )?;

    let mut read_packets = Vec::new();
    let mut packet_index = 0;

    while let Some(packet) = reader.read_packet()? {
        read_packets.push(PacketInfo {
            index: packet_index,
            packet_length: packet.packet_length() as u32,
            checksum: packet.checksum(),
            first_bytes: packet
                .data
                .iter()
                .take(16)
                .cloned()
                .collect(),
        });
        packet_index += 1;
    }

    Ok(read_packets)
}

/// 验证数据一致性
fn validate_data_consistency(
    written: &[PacketInfo],
    read: &[PacketInfo],
) -> bool {
    if written.len() != read.len() {
        return false;
    }

    for i in 0..written.len() {
        let w = &written[i];
        let r = &read[i];

        if w.index != r.index
            || w.packet_length != r.packet_length
            || w.checksum != r.checksum
            || w.first_bytes != r.first_bytes
        {
            return false;
        }
    }

    true
}

#[test]
fn test_small_dataset_basic_functionality() {
    // 创建临时目录
    let temp_dir =
        TempDir::new().expect("创建临时目录失败");
    let base_path = temp_dir.path();
    let dataset_name = "small_test_dataset";

    const PACKET_COUNT: usize = 2000;
    const PACKET_SIZE: usize = 1024;

    // 步骤1: 写入测试数据
    let written_packets = write_small_dataset(
        base_path,
        dataset_name,
        PACKET_COUNT,
        PACKET_SIZE,
    )
    .expect("写入测试数据失败");

    assert_eq!(
        written_packets.len(),
        PACKET_COUNT,
        "写入的数据包数量不正确"
    );

    // 步骤2: 读取测试数据
    let read_packets =
        read_test_packets(base_path, dataset_name)
            .expect("读取测试数据失败");

    assert_eq!(
        read_packets.len(),
        PACKET_COUNT,
        "读取的数据包数量不正确"
    );

    // 步骤3: 验证数据一致性
    let is_valid = validate_data_consistency(
        &written_packets,
        &read_packets,
    );
    assert!(is_valid, "数据一致性验证失败");

    println!(
        "✅ 小规模数据集测试通过：{} 个数据包",
        PACKET_COUNT
    );
}

#[test]
fn test_small_dataset_multiple_files() {
    // 测试数据包分割到多个文件的功能
    let temp_dir =
        TempDir::new().expect("创建临时目录失败");
    let base_path = temp_dir.path();
    let dataset_name = "multi_file_test";

    // 使用较小的文件大小限制来强制分割
    let mut config = WriterConfig::default();
    config.max_packets_per_file = 100; // 每个文件最多100个数据包

    let mut writer = PcapWriter::new_with_config(
        base_path,
        dataset_name,
        config,
    )
    .expect("创建PcapWriter失败");

    const PACKET_COUNT: usize = 350; // 应该分割为4个文件
    const PACKET_SIZE: usize = 512;

    // 写入数据包
    for i in 0..PACKET_COUNT {
        let packet = create_test_packet(i, PACKET_SIZE)
            .expect("创建数据包失败");
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");
    }

    writer.finalize().expect("完成写入失败");

    // 验证读取
    let mut reader =
        PcapReader::new(base_path, dataset_name)
            .expect("创建PcapReader失败");

    let dataset_info = reader
        .get_dataset_info()
        .expect("获取数据集信息失败");
    assert_eq!(
        dataset_info.total_packets, PACKET_COUNT as u64,
        "总数据包数量不正确"
    );
    assert!(
        dataset_info.file_count >= 4,
        "文件分割数量不正确，期望至少4个文件"
    );

    // 验证能读取所有数据包
    let mut read_count = 0;
    while let Some(_packet) =
        reader.read_packet().expect("读取数据包失败")
    {
        read_count += 1;
    }

    assert_eq!(
        read_count, PACKET_COUNT,
        "读取的数据包总数不正确"
    );

    println!(
        "✅ 多文件分割测试通过：{} 个数据包分割为 {} 个文件",
        PACKET_COUNT, dataset_info.file_count
    );
}

#[test]
fn test_small_dataset_empty_and_edge_cases() {
    let temp_dir =
        TempDir::new().expect("创建临时目录失败");
    let base_path = temp_dir.path();

    // 测试空数据集
    let dataset_name = "empty_test";
    let mut writer = PcapWriter::new_with_config(
        base_path,
        dataset_name,
        WriterConfig::default(),
    )
    .expect("创建PcapWriter失败");
    writer.finalize().expect("完成空写入失败");

    // 尝试读取空数据集
    let mut reader =
        PcapReader::new(base_path, dataset_name)
            .expect("创建PcapReader失败");

    let dataset_info = reader.get_dataset_info().expect("获取数据集信息失败");
    assert_eq!(
        dataset_info.total_packets, 0,
        "空数据集应该有0个数据包"
    );

    let packet =
        reader.read_packet().expect("读取空数据集失败");
    assert!(packet.is_none(), "空数据集应该返回None");

    println!("✅ 空数据集和边界情况测试通过");
}
