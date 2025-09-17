//! 自动索引生成测试
//!
//! 测试无索引写入和读取是否能自动生成索引并验证索引的正确性

use pcapfile_io::{
    PcapReader, PcapWriter, ReaderConfig, WriterConfig,
};

mod common;
use common::{create_test_packet, setup_test_environment};

#[test]
fn test_auto_index_with_small_dataset() {
    const TEST_NAME: &str =
        "test_auto_index_with_small_dataset";
    let dataset_path = setup_test_environment(TEST_NAME)
        .expect("设置测试环境失败");

    const PACKET_COUNT: usize = 500;
    const PACKET_SIZE: usize = 64;

    // 步骤1: 创建启用自动索引的写入器
    let config = WriterConfig::default();

    let mut writer = PcapWriter::new_with_config(
        &dataset_path,
        TEST_NAME,
        config,
    )
    .expect("创建PcapWriter失败");

    for i in 0..PACKET_COUNT {
        let packet =
            create_test_packet(i as u32, PACKET_SIZE)
                .expect("创建测试数据包失败");
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");
    }

    writer.finalize().expect("完成写入失败");

    // 步骤2: 使用Reader验证自动生成的索引
    let mut reader = PcapReader::new_with_config(
        &dataset_path,
        TEST_NAME,
        ReaderConfig::default(),
    )
    .expect("创建PcapReader失败");
    reader.initialize().expect("初始化Reader失败");

    // 通过reader.index()访问索引
    let index =
        reader.index().get_index().expect("获取索引失败");

    assert_eq!(index.total_packets, PACKET_COUNT as u64);
    assert!(!index.data_files.files.is_empty());
    assert_eq!(index.timestamp_index.len(), PACKET_COUNT);

    println!("小数据集自动索引测试通过");
}

#[test]
fn test_auto_index_with_multiple_files() {
    const TEST_NAME: &str =
        "test_auto_index_with_multiple_files";
    let dataset_path = setup_test_environment(TEST_NAME)
        .expect("设置测试环境失败");

    const TOTAL_PACKETS: usize = 3000;
    const PACKET_SIZE: usize = 128;

    // 配置写入器生成多个文件
    let config = WriterConfig {
        max_packets_per_file: 1000, // 每1000个数据包一个文件
        ..Default::default()
    };

    let mut writer = PcapWriter::new_with_config(
        &dataset_path,
        TEST_NAME,
        config,
    )
    .expect("创建PcapWriter失败");

    for i in 0..TOTAL_PACKETS {
        let packet =
            create_test_packet(i as u32, PACKET_SIZE)
                .expect("创建测试数据包失败");
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");
    }

    writer.finalize().expect("完成写入失败");

    // 验证自动生成的索引
    let mut reader = PcapReader::new_with_config(
        &dataset_path,
        TEST_NAME,
        ReaderConfig::default(),
    )
    .expect("创建PcapReader失败");
    reader.initialize().expect("初始化Reader失败");

    let index =
        reader.index().get_index().expect("获取索引失败");

    assert_eq!(index.total_packets, TOTAL_PACKETS as u64);
    // 应该有3个文件：1000 + 1000 + 1000
    assert_eq!(index.data_files.files.len(), 3);

    println!("多文件自动索引测试通过");
}

#[test]
fn test_manual_index_generation_after_write() {
    const TEST_NAME: &str =
        "test_manual_index_generation_after_write";
    let dataset_path = setup_test_environment(TEST_NAME)
        .expect("设置测试环境失败");

    const PACKET_COUNT: usize = 1500;
    const PACKET_SIZE: usize = 256;

    // 步骤1: 禁用自动索引，仅写入数据
    let config = WriterConfig::default();

    let mut writer = PcapWriter::new_with_config(
        &dataset_path,
        TEST_NAME,
        config,
    )
    .expect("创建PcapWriter失败");

    for i in 0..PACKET_COUNT {
        let packet =
            create_test_packet(i as u32, PACKET_SIZE)
                .expect("创建测试数据包失败");
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");
    }

    writer.finalize().expect("完成写入失败");

    // 步骤2: 手动生成索引
    let mut reader = PcapReader::new_with_config(
        &dataset_path,
        TEST_NAME,
        ReaderConfig::default(),
    )
    .expect("创建PcapReader失败");

    // 生成索引
    let index_path = reader
        .index_mut()
        .regenerate_index()
        .expect("手动生成索引失败");

    // 步骤3: 验证手动生成的索引
    let index =
        reader.index().get_index().expect("获取索引失败");

    assert_eq!(index.total_packets, PACKET_COUNT as u64);
    assert!(!index.data_files.files.is_empty());

    println!("手动索引生成测试通过: {index_path:?}");
}

#[test]
fn test_index_consistency_check() {
    const TEST_NAME: &str = "test_index_consistency_check";
    let dataset_path = setup_test_environment(TEST_NAME)
        .expect("设置测试环境失败");

    const PACKET_COUNT: usize = 2000;
    const PACKET_SIZE: usize = 200;

    // 创建数据集
    let mut writer = PcapWriter::new_with_config(
        &dataset_path,
        TEST_NAME,
        WriterConfig::default(),
    )
    .expect("创建PcapWriter失败");

    let mut expected_timestamps = Vec::new();
    for i in 0..PACKET_COUNT {
        let packet =
            create_test_packet(i as u32, PACKET_SIZE)
                .expect("创建测试数据包失败");
        expected_timestamps.push(packet.get_timestamp_ns());
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");
    }

    writer.finalize().expect("完成写入失败");

    // 验证索引一致性
    let mut reader = PcapReader::new_with_config(
        &dataset_path,
        TEST_NAME,
        ReaderConfig::default(),
    )
    .expect("创建PcapReader失败");
    reader.initialize().expect("初始化Reader失败");

    let index =
        reader.index().get_index().expect("获取索引失败");

    // 验证数据包总数
    assert_eq!(index.total_packets, PACKET_COUNT as u64);

    // 验证时间戳索引
    assert_eq!(index.timestamp_index.len(), PACKET_COUNT);

    // 验证时间戳一致性
    for expected_ts in &expected_timestamps {
        assert!(
            index.timestamp_index.contains_key(expected_ts),
            "索引中缺少时间戳: {expected_ts}"
        );
    }

    // 验证索引不需要重建
    let needs_rebuild = reader
        .index()
        .needs_rebuild()
        .expect("检查重建状态失败");
    assert!(!needs_rebuild, "新生成的索引不应该需要重建");

    println!("索引一致性检查通过");
}
