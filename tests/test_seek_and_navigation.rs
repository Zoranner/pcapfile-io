//! 测试新增的定位和导航接口

use pcapfile_io::{
    DataPacket, PcapReader, PcapWriter, WriterConfig,
};
use std::path::Path;

mod common;
use common::{
    clean_dataset_directory, setup_test_environment,
};

/// 创建测试数据集的辅助函数
fn create_test_dataset(
    base_path: &Path,
    dataset_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 清理测试数据集目录
    let test_dataset_path = base_path.join(dataset_name);
    clean_dataset_directory(&test_dataset_path)?;

    let config = WriterConfig {
        max_packets_per_file: 50,
        ..Default::default()
    };

    let mut writer = PcapWriter::new_with_config(
        base_path,
        dataset_name,
        config,
    )?;
    writer.initialize()?;

    // 写入100个数据包
    for i in 0..100 {
        let timestamp_ns = 1_000_000_000 + (i * 10_000_000); // 每10ms一个包
        let timestamp_sec =
            (timestamp_ns / 1_000_000_000) as u32;
        let timestamp_nsec =
            (timestamp_ns % 1_000_000_000) as u32;
        let data =
            format!("Test packet {}", i).into_bytes();
        let packet = DataPacket::from_timestamp(
            timestamp_sec,
            timestamp_nsec,
            data,
        )?;
        writer.write_packet(&packet)?;
    }

    writer.finalize()?;
    Ok(())
}

#[test]
fn test_state_query_interfaces() {
    const TEST_NAME: &str = "test_state_query";
    let base_path =
        setup_test_environment().expect("设置测试环境失败");

    // 创建测试数据
    create_test_dataset(&base_path, TEST_NAME)
        .expect("创建测试数据集失败");

    let mut reader = PcapReader::new(&base_path, TEST_NAME)
        .expect("创建Reader失败");
    reader.initialize().expect("初始化失败");

    // 测试状态查询接口
    assert_eq!(reader.total_packets(), Some(100));
    assert_eq!(reader.current_packet_index(), 0);
    assert!(!reader.is_eof());
    assert_eq!(reader.progress(), Some(0.0));
}

#[test]
fn test_seek_to_packet() {
    const TEST_NAME: &str = "test_seek_packet";
    let base_path =
        setup_test_environment().expect("设置测试环境失败");

    create_test_dataset(&base_path, TEST_NAME)
        .expect("创建测试数据集失败");

    let mut reader = PcapReader::new(&base_path, TEST_NAME)
        .expect("创建Reader失败");
    reader.initialize().expect("初始化失败");

    // 测试跳转到指定数据包
    reader.seek_to_packet(50).expect("跳转失败");
    assert_eq!(reader.current_packet_index(), 50);

    // 读取该数据包验证时间戳
    if let Some(packet) =
        reader.read_packet_data_only().expect("读取失败")
    {
        let expected_ts = 1_000_000_000 + (50 * 10_000_000);
        assert_eq!(packet.get_timestamp_ns(), expected_ts);
    } else {
        panic!("未能读取到数据包");
    }

    // 测试边界：跳转到第一个数据包
    reader.seek_to_packet(0).expect("跳转到开头失败");
    assert_eq!(reader.current_packet_index(), 0);

    // 测试边界：跳转到最后一个数据包
    reader.seek_to_packet(99).expect("跳转到末尾失败");
    assert_eq!(reader.current_packet_index(), 99);

    // 测试超出范围
    let result = reader.seek_to_packet(100);
    assert!(result.is_err(), "应该返回错误");
}

#[test]
fn test_skip_packets() {
    const TEST_NAME: &str = "test_skip_packets";
    let base_path =
        setup_test_environment().expect("设置测试环境失败");

    create_test_dataset(&base_path, TEST_NAME)
        .expect("创建测试数据集失败");

    let mut reader = PcapReader::new(&base_path, TEST_NAME)
        .expect("创建Reader失败");
    reader.initialize().expect("初始化失败");

    // 从开头跳过10个
    let skipped =
        reader.skip_packets(10).expect("跳过失败");
    assert_eq!(skipped, 10);
    assert_eq!(reader.current_packet_index(), 10);

    // 再跳过20个
    let skipped =
        reader.skip_packets(20).expect("跳过失败");
    assert_eq!(skipped, 20);
    assert_eq!(reader.current_packet_index(), 30);

    // 尝试跳过超过剩余数量的包
    let skipped =
        reader.skip_packets(100).expect("跳过失败");
    // 从索引30开始，最多能跳到索引99（总共100个包，索引0-99）
    // 实际跳过：99 - 30 = 69
    assert_eq!(skipped, 69);
    assert_eq!(reader.current_packet_index(), 99);
}

#[test]
fn test_seek_to_timestamp() {
    const TEST_NAME: &str = "test_seek_timestamp";
    let base_path =
        setup_test_environment().expect("设置测试环境失败");

    create_test_dataset(&base_path, TEST_NAME)
        .expect("创建测试数据集失败");

    let mut reader = PcapReader::new(&base_path, TEST_NAME)
        .expect("创建Reader失败");
    reader.initialize().expect("初始化失败");

    // 测试精确匹配
    let target_timestamp =
        1_000_000_000 + (75 * 10_000_000);
    let actual_ts = reader
        .seek_to_timestamp(target_timestamp)
        .expect("跳转失败");
    assert_eq!(actual_ts, target_timestamp);
    assert_eq!(reader.current_packet_index(), 75);

    // 验证读取的数据包
    if let Some(packet) =
        reader.read_packet_data_only().expect("读取失败")
    {
        assert_eq!(
            packet.get_timestamp_ns(),
            target_timestamp
        );
    } else {
        panic!("未能读取到数据包");
    }

    // 测试不精确时间戳（应该返回 >= 目标的最小值）
    let non_exact_ts =
        1_000_000_000 + (80 * 10_000_000) + 5_000_000; // 介于两个包之间
    let actual_ts = reader
        .seek_to_timestamp(non_exact_ts)
        .expect("跳转失败");
    let expected_ts = 1_000_000_000 + (81 * 10_000_000); // 应该定位到下一个包
    assert_eq!(actual_ts, expected_ts);
    assert_eq!(reader.current_packet_index(), 81);
}

#[test]
fn test_is_eof() {
    const TEST_NAME: &str = "test_eof";
    let base_path =
        setup_test_environment().expect("设置测试环境失败");

    create_test_dataset(&base_path, TEST_NAME)
        .expect("创建测试数据集失败");

    let mut reader = PcapReader::new(&base_path, TEST_NAME)
        .expect("创建Reader失败");
    reader.initialize().expect("初始化失败");

    // 开始时不应该是EOF
    assert!(!reader.is_eof());

    // 跳转到最后一个数据包
    reader.seek_to_packet(99).expect("跳转失败");
    assert!(!reader.is_eof()); // 还没读取，不是EOF

    // 读取最后一个包
    reader.read_packet_data_only().expect("读取失败");
    assert!(reader.is_eof()); // 现在应该是EOF
}

#[test]
fn test_progress() {
    const TEST_NAME: &str = "test_progress";
    let base_path =
        setup_test_environment().expect("设置测试环境失败");

    create_test_dataset(&base_path, TEST_NAME)
        .expect("创建测试数据集失败");

    let mut reader = PcapReader::new(&base_path, TEST_NAME)
        .expect("创建Reader失败");
    reader.initialize().expect("初始化失败");

    // 开始时进度应该是0
    assert_eq!(reader.progress(), Some(0.0));

    // 跳转到50%
    reader.seek_to_packet(50).expect("跳转失败");
    assert_eq!(reader.progress(), Some(0.5));

    // 跳转到75%
    reader.seek_to_packet(75).expect("跳转失败");
    assert_eq!(reader.progress(), Some(0.75));

    // 跳转到最后一个数据包（索引99）
    reader.seek_to_packet(99).expect("跳转失败");
    assert_eq!(reader.progress(), Some(0.99));
}

#[test]
fn test_reset_after_seek() {
    const TEST_NAME: &str = "test_reset";
    let base_path =
        setup_test_environment().expect("设置测试环境失败");

    create_test_dataset(&base_path, TEST_NAME)
        .expect("创建测试数据集失败");

    let mut reader = PcapReader::new(&base_path, TEST_NAME)
        .expect("创建Reader失败");
    reader.initialize().expect("初始化失败");

    // 跳转到中间
    reader.seek_to_packet(50).expect("跳转失败");
    assert_eq!(reader.current_packet_index(), 50);

    // 重置
    reader.reset().expect("重置失败");
    assert_eq!(reader.current_packet_index(), 0);
    assert_eq!(reader.progress(), Some(0.0));
    assert!(!reader.is_eof());
}

#[test]
fn test_combined_navigation() {
    const TEST_NAME: &str = "test_combined";
    let base_path =
        setup_test_environment().expect("设置测试环境失败");

    create_test_dataset(&base_path, TEST_NAME)
        .expect("创建测试数据集失败");

    let mut reader = PcapReader::new(&base_path, TEST_NAME)
        .expect("创建Reader失败");
    reader.initialize().expect("初始化失败");

    // 读取几个包
    for _ in 0..3 {
        reader.read_packet_data_only().expect("读取失败");
    }
    assert_eq!(reader.current_packet_index(), 3);

    // 跳转到索引
    reader.seek_to_packet(50).expect("跳转失败");
    assert_eq!(reader.current_packet_index(), 50);

    // 跳过10个
    reader.skip_packets(10).expect("跳过失败");
    assert_eq!(reader.current_packet_index(), 60);

    // 按时间戳跳转
    let target_ts = 1_000_000_000 + (30 * 10_000_000);
    reader.seek_to_timestamp(target_ts).expect("跳转失败");
    assert_eq!(reader.current_packet_index(), 30);

    // 重置
    reader.reset().expect("重置失败");
    assert_eq!(reader.current_packet_index(), 0);
}
