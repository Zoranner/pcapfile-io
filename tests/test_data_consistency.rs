//! 数据一致性测试
//!
//! 测试写入和读取的一致性，确保数据完整性和可靠性

use pcap_io::{DataPacket, PcapReader, PcapWriter, PcapResult};
use std::path::Path;
use std::time::SystemTime;
use tempfile::TempDir;

/// 数据包详细信息结构
#[derive(Debug, Clone, PartialEq)]
struct PacketDetails {
    index: usize,
    timestamp_ns: u64,
    packet_length: u32,
    checksum: u32,
    data_hash: String, // 数据内容的哈希值
    first_16_bytes: Vec<u8>,
    last_16_bytes: Vec<u8>,
}

/// 计算数据哈希
fn calculate_data_hash(data: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// 创建具有特定模式的测试数据包
fn create_detailed_test_packet(sequence: usize, size: usize) -> PcapResult<DataPacket> {
    let mut data = vec![0u8; size];

    // 创建具有清晰模式的数据，以便检测损坏
    for i in 0..size {
        data[i] = match i % 4 {
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

/// 从数据包创建详细信息
fn create_packet_details(packet: &DataPacket, index: usize) -> PacketDetails {
    let data_hash = calculate_data_hash(&packet.data);
    let first_16_bytes = packet.data.iter().take(16).cloned().collect();
    let last_16_bytes = packet.data.iter().rev().take(16).cloned().collect();

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

/// 写入测试数据包并返回详细信息
fn write_detailed_test_data(
    base_path: &Path,
    dataset_name: &str,
    packet_count: usize,
    packet_size: usize,
) -> PcapResult<Vec<PacketDetails>> {
    let mut writer = PcapWriter::new(base_path, dataset_name)?;

    let mut written_details = Vec::with_capacity(packet_count);

    for i in 0..packet_count {
        let packet = create_detailed_test_packet(i, packet_size)?;
        let details = create_packet_details(&packet, i);

        writer.write_packet(&packet)?;
        written_details.push(details);
    }

    writer.finalize()?;
    Ok(written_details)
}

/// 读取并验证数据包
fn read_and_verify_test_data(base_path: &Path, dataset_name: &str) -> PcapResult<Vec<PacketDetails>> {
    let mut reader = PcapReader::new(base_path, dataset_name)?;

    let mut read_details = Vec::new();
    let mut packet_index = 0;

    while let Some(packet) = reader.read_packet()? {
        let details = create_packet_details(&packet, packet_index);
        read_details.push(details);
        packet_index += 1;
    }

    Ok(read_details)
}

/// 深度比较两个数据包详细信息列表
fn deep_compare_packet_details(
    written: &[PacketDetails],
    read: &[PacketDetails],
) -> (bool, Vec<String>) {
    let mut errors = Vec::new();
    let mut is_consistent = true;

    // 检查数量
    if written.len() != read.len() {
        errors.push(format!(
            "数据包数量不匹配：写入 {}，读取 {}",
            written.len(),
            read.len()
        ));
        is_consistent = false;
    }

    // 逐个比较数据包
    let min_count = written.len().min(read.len());
    for i in 0..min_count {
        let w = &written[i];
        let r = &read[i];

        if w != r {
            is_consistent = false;

            if w.index != r.index {
                errors.push(format!(
                    "数据包 {}: 索引不匹配 (写入: {}, 读取: {})",
                    i, w.index, r.index
                ));
            }

            if w.timestamp_ns != r.timestamp_ns {
                errors.push(format!(
                    "数据包 {}: 时间戳不匹配 (写入: {}, 读取: {})",
                    i, w.timestamp_ns, r.timestamp_ns
                ));
            }

            if w.packet_length != r.packet_length {
                errors.push(format!(
                    "数据包 {}: 长度不匹配 (写入: {}, 读取: {})",
                    i, w.packet_length, r.packet_length
                ));
            }

            if w.checksum != r.checksum {
                errors.push(format!(
                    "数据包 {}: 校验和不匹配 (写入: 0x{:08X}, 读取: 0x{:08X})",
                    i, w.checksum, r.checksum
                ));
            }

            if w.data_hash != r.data_hash {
                errors.push(format!(
                    "数据包 {}: 数据哈希不匹配 (写入: {}, 读取: {})",
                    i, w.data_hash, r.data_hash
                ));
            }

            if w.first_16_bytes != r.first_16_bytes {
                errors.push(format!("数据包 {}: 前16字节不匹配", i));
            }

            if w.last_16_bytes != r.last_16_bytes {
                errors.push(format!("数据包 {}: 后16字节不匹配", i));
            }
        }
    }

    (is_consistent, errors)
}

#[test]
fn test_basic_data_consistency() {
    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let base_path = temp_dir.path();
    let dataset_name = "consistency_test";

    const PACKET_COUNT: usize = 1000;
    const PACKET_SIZE: usize = 1024;

    // 写入测试数据
    let written_details =
        write_detailed_test_data(base_path, dataset_name, PACKET_COUNT, PACKET_SIZE)
            .expect("写入测试数据失败");

    // 读取并验证数据
    let read_details =
        read_and_verify_test_data(base_path, dataset_name).expect("读取测试数据失败");

    // 深度比较
    let (is_consistent, errors) = deep_compare_packet_details(&written_details, &read_details);

    if !is_consistent {
        for error in errors.iter().take(10) {
            println!("错误: {}", error);
        }
        if errors.len() > 10 {
            println!("... 还有 {} 个错误", errors.len() - 10);
        }
    }

    assert!(
        is_consistent,
        "数据一致性测试失败，发现 {} 个错误",
        errors.len()
    );

    println!(
        "✅ 基本数据一致性测试通过：{} 个数据包完全一致",
        PACKET_COUNT
    );
}

#[test]
fn test_large_packet_consistency() {
    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let base_path = temp_dir.path();
    let dataset_name = "large_packet_consistency";

    const PACKET_COUNT: usize = 100;
    const PACKET_SIZE: usize = 64 * 1024; // 64KB大数据包

    // 写入大数据包
    let written_details =
        write_detailed_test_data(base_path, dataset_name, PACKET_COUNT, PACKET_SIZE)
            .expect("写入大数据包失败");

    // 读取并验证
    let read_details =
        read_and_verify_test_data(base_path, dataset_name).expect("读取大数据包失败");

    // 验证一致性
    let (is_consistent, errors) = deep_compare_packet_details(&written_details, &read_details);

    assert!(is_consistent, "大数据包一致性测试失败，错误: {:?}", errors);

    println!(
        "✅ 大数据包一致性测试通过：{} 个 {}KB 数据包",
        PACKET_COUNT,
        PACKET_SIZE / 1024
    );
}

#[test]
fn test_mixed_size_packet_consistency() {
    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let base_path = temp_dir.path();
    let dataset_name = "mixed_size_consistency";

    // 创建不同大小的数据包
    let packet_sizes = vec![64, 256, 512, 1024, 2048, 4096, 8192];
    let packets_per_size = 50;

    let mut writer =
        PcapWriter::new(base_path, dataset_name).expect("创建Writer失败");

    let mut written_details = Vec::new();
    let mut packet_index = 0;

    for &size in &packet_sizes {
        for i in 0..packets_per_size {
            let packet =
                create_detailed_test_packet(packet_index + i, size).expect("创建数据包失败");
            let details = create_packet_details(&packet, packet_index);

            writer.write_packet(&packet).expect("写入数据包失败");
            written_details.push(details);
            packet_index += 1;
        }
    }

    writer.finalize().expect("完成写入失败");

    // 读取并验证
    let read_details =
        read_and_verify_test_data(base_path, dataset_name).expect("读取混合大小数据包失败");

    // 验证一致性
    let (is_consistent, errors) = deep_compare_packet_details(&written_details, &read_details);

    assert!(
        is_consistent,
        "混合大小数据包一致性测试失败，错误: {:?}",
        errors
    );

    let total_packets = packet_sizes.len() * packets_per_size;
    println!(
        "✅ 混合大小数据包一致性测试通过：{} 个不同大小数据包",
        total_packets
    );
}

#[test]
fn test_timestamp_consistency() {
    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let base_path = temp_dir.path();
    let dataset_name = "timestamp_consistency";

    const PACKET_COUNT: usize = 500;
    const PACKET_SIZE: usize = 512;

    // 写入数据包，记录时间戳
    let mut writer =
        PcapWriter::new(base_path, dataset_name).expect("创建Writer失败");

    let mut written_timestamps = Vec::new();

    for i in 0..PACKET_COUNT {
        let packet = create_detailed_test_packet(i, PACKET_SIZE).expect("创建数据包失败");
        written_timestamps.push(packet.get_timestamp_ns());
        writer.write_packet(&packet).expect("写入数据包失败");

        // 添加小延迟确保时间戳不同
        std::thread::sleep(std::time::Duration::from_micros(1));
    }

    writer.finalize().expect("完成写入失败");

    // 读取并验证时间戳
    let mut reader = PcapReader::new(base_path, dataset_name).expect("创建Reader失败");

    let mut read_timestamps = Vec::new();
    while let Some(packet) = reader.read_packet().expect("读取数据包失败") {
        read_timestamps.push(packet.get_timestamp_ns());
    }

    // 验证时间戳完全一致
    assert_eq!(
        written_timestamps.len(),
        read_timestamps.len(),
        "时间戳数量不匹配"
    );

    for (i, (&written_ts, &read_ts)) in written_timestamps
        .iter()
        .zip(read_timestamps.iter())
        .enumerate()
    {
        assert_eq!(
            written_ts, read_ts,
            "数据包 {} 时间戳不匹配：写入 {}，读取 {}",
            i, written_ts, read_ts
        );
    }

    println!(
        "✅ 时间戳一致性测试通过：{} 个数据包时间戳完全匹配",
        PACKET_COUNT
    );
}
