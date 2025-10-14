//! 时间戳索引功能测试
//!
//! 测试基于时间戳的索引查找、时间范围读取、跨文件随机访问等功能

use pcapfile_io::{
    PcapReader, PcapWriter, ReaderConfig, WriterConfig,
};
use std::time::Duration;

mod common;
use common::{
    clean_dataset_directory, create_test_packet,
    setup_test_environment,
};

#[test]
fn test_read_packets_by_time_range() {
    const TEST_NAME: &str =
        "test_read_packets_by_time_range";
    let dataset_path =
        setup_test_environment().expect("设置测试环境失败");

    // 清理测试数据集目录
    let test_dataset_path = dataset_path.join(TEST_NAME);
    clean_dataset_directory(&test_dataset_path)
        .expect("清理测试目录失败");

    const PACKET_COUNT: usize = 1000;
    const PACKET_SIZE: usize = 128;

    // 创建多文件数据集
    let config = WriterConfig {
        max_packets_per_file: 300, // 每300个数据包一个文件
        ..Default::default()
    };

    let mut writer = PcapWriter::new_with_config(
        &dataset_path,
        TEST_NAME,
        config,
    )
    .expect("创建PcapWriter失败");

    let mut written_timestamps = Vec::new();
    for i in 0..PACKET_COUNT {
        let packet =
            create_test_packet(i as u32, PACKET_SIZE)
                .expect("创建测试数据包失败");
        written_timestamps.push(packet.get_timestamp_ns());
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");

        // 添加小延迟确保时间戳递增
        std::thread::sleep(Duration::from_micros(10));
    }

    writer.finalize().expect("完成写入失败");

    // 测试时间范围读取
    let mut reader = PcapReader::new_with_config(
        &dataset_path,
        TEST_NAME,
        ReaderConfig::default(),
    )
    .expect("创建PcapReader失败");
    reader.initialize().expect("初始化Reader失败");

    // 选择中间的时间范围
    let start_timestamp = written_timestamps[200];
    let end_timestamp = written_timestamps[800];

    let range_packets = reader
        .read_packets_by_time_range(
            start_timestamp,
            end_timestamp,
        )
        .expect("时间范围读取失败");

    // 验证结果
    assert_eq!(range_packets.len(), 601); // 200到800，包含两端

    // 验证时间戳顺序
    for i in 1..range_packets.len() {
        assert!(
            range_packets[i].packet.get_timestamp_ns()
                >= range_packets[i - 1]
                    .packet
                    .get_timestamp_ns(),
            "时间戳顺序错误"
        );
    }

    // 验证时间戳范围
    for packet in &range_packets {
        let ts = packet.packet.get_timestamp_ns();
        assert!(
            ts >= start_timestamp && ts <= end_timestamp,
            "时间戳超出范围: {ts}"
        );
    }

    println!(
        "✅ 时间范围读取测试通过：{} 个数据包",
        range_packets.len()
    );
}

#[test]
fn test_read_packet_by_timestamp() {
    const TEST_NAME: &str = "test_read_packet_by_timestamp";
    let dataset_path =
        setup_test_environment().expect("设置测试环境失败");

    // 清理测试数据集目录
    let test_dataset_path = dataset_path.join(TEST_NAME);
    clean_dataset_directory(&test_dataset_path)
        .expect("清理测试目录失败");

    const PACKET_COUNT: usize = 500;
    const PACKET_SIZE: usize = 256;

    // 创建数据集
    let mut writer = PcapWriter::new_with_config(
        &dataset_path,
        TEST_NAME,
        WriterConfig::default(),
    )
    .expect("创建PcapWriter失败");

    let mut written_timestamps = Vec::new();
    for i in 0..PACKET_COUNT {
        let packet =
            create_test_packet(i as u32, PACKET_SIZE)
                .expect("创建测试数据包失败");
        written_timestamps.push(packet.get_timestamp_ns());
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");

        // 添加小延迟确保时间戳递增
        std::thread::sleep(Duration::from_micros(5));
    }

    writer.finalize().expect("完成写入失败");

    // 测试精确时间戳读取
    let mut reader = PcapReader::new_with_config(
        &dataset_path,
        TEST_NAME,
        ReaderConfig::default(),
    )
    .expect("创建PcapReader失败");
    reader.initialize().expect("初始化Reader失败");

    // 测试几个不同的时间戳
    let test_indices = [0, 100, 250, 400, 499];

    for &index in &test_indices {
        let target_timestamp = written_timestamps[index];

        let packet = reader
            .read_packet_by_timestamp(target_timestamp)
            .expect("精确时间戳读取失败")
            .expect("未找到指定时间戳的数据包");

        assert_eq!(
            packet.packet.get_timestamp_ns(),
            target_timestamp,
            "时间戳不匹配"
        );

        // 验证数据包内容
        assert_eq!(
            packet.packet.packet_length() as usize,
            PACKET_SIZE,
            "数据包大小不匹配"
        );
    }

    // 测试不存在的时间戳
    let non_existent_timestamp =
        written_timestamps[0] - 1000;
    let result = reader
        .read_packet_by_timestamp(non_existent_timestamp)
        .expect("读取不存在的时间戳失败");

    assert!(result.is_none(), "应该返回None");

    println!(
        "✅ 精确时间戳读取测试通过：{} 个测试点",
        test_indices.len()
    );
}

#[test]
fn test_seek_by_timestamp() {
    const TEST_NAME: &str = "test_seek_by_timestamp";
    let dataset_path =
        setup_test_environment().expect("设置测试环境失败");

    // 清理测试数据集目录
    let test_dataset_path = dataset_path.join(TEST_NAME);
    clean_dataset_directory(&test_dataset_path)
        .expect("清理测试目录失败");

    const PACKET_COUNT: usize = 300;
    const PACKET_SIZE: usize = 64;

    // 创建数据集
    let mut writer = PcapWriter::new_with_config(
        &dataset_path,
        TEST_NAME,
        WriterConfig::default(),
    )
    .expect("创建PcapWriter失败");

    let mut written_timestamps = Vec::new();
    for i in 0..PACKET_COUNT {
        let packet =
            create_test_packet(i as u32, PACKET_SIZE)
                .expect("创建测试数据包失败");
        written_timestamps.push(packet.get_timestamp_ns());
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");

        // 添加小延迟确保时间戳递增
        std::thread::sleep(Duration::from_micros(3));
    }

    writer.finalize().expect("完成写入失败");

    // 测试时间戳定位
    let mut reader = PcapReader::new_with_config(
        &dataset_path,
        TEST_NAME,
        ReaderConfig::default(),
    )
    .expect("创建PcapReader失败");
    reader.initialize().expect("初始化Reader失败");

    // 测试精确匹配
    let exact_timestamp = written_timestamps[150];
    let pointer = reader
        .seek_by_timestamp(exact_timestamp)
        .expect("时间戳定位失败")
        .expect("未找到时间戳");

    assert_eq!(
        pointer.entry.timestamp_ns, exact_timestamp,
        "定位的时间戳不匹配"
    );

    // 测试近似匹配
    let approximate_timestamp =
        written_timestamps[75] + 100; // 稍微偏移
    let pointer = reader
        .seek_by_timestamp(approximate_timestamp)
        .expect("近似时间戳定位失败")
        .expect("未找到最接近的时间戳");

    // 验证找到的是最接近的时间戳
    let found_timestamp = pointer.entry.timestamp_ns;
    let expected_timestamp = written_timestamps[75];

    assert_eq!(
        found_timestamp, expected_timestamp,
        "找到的不是最接近的时间戳"
    );

    // 验证时间差
    let diff =
        found_timestamp.abs_diff(approximate_timestamp);
    assert!(diff <= 100, "时间差过大: {diff}");

    println!("✅ 时间戳定位测试通过");
}

#[test]
fn test_cross_file_random_access() {
    const TEST_NAME: &str = "test_cross_file_random_access";
    let dataset_path =
        setup_test_environment().expect("设置测试环境失败");

    // 清理测试数据集目录
    let test_dataset_path = dataset_path.join(TEST_NAME);
    clean_dataset_directory(&test_dataset_path)
        .expect("清理测试目录失败");

    const PACKET_COUNT: usize = 900;
    const PACKET_SIZE: usize = 512;

    // 创建多文件数据集
    let config = WriterConfig {
        max_packets_per_file: 200, // 每200个数据包一个文件，会产生5个文件
        ..Default::default()
    };

    let mut writer = PcapWriter::new_with_config(
        &dataset_path,
        TEST_NAME,
        config,
    )
    .expect("创建PcapWriter失败");

    let mut written_timestamps = Vec::new();
    for i in 0..PACKET_COUNT {
        let packet =
            create_test_packet(i as u32, PACKET_SIZE)
                .expect("创建测试数据包失败");
        written_timestamps.push(packet.get_timestamp_ns());
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");

        // 添加小延迟确保时间戳递增
        std::thread::sleep(Duration::from_micros(2));
    }

    writer.finalize().expect("完成写入失败");

    // 测试跨文件随机访问
    let mut reader = PcapReader::new_with_config(
        &dataset_path,
        TEST_NAME,
        ReaderConfig::default(),
    )
    .expect("创建PcapReader失败");
    reader.initialize().expect("初始化Reader失败");

    // 测试不同文件中的数据包
    let test_indices = [50, 250, 450, 650, 850]; // 分布在不同的文件中

    for &index in &test_indices {
        let target_timestamp = written_timestamps[index];

        let packet = reader
            .read_packet_by_timestamp(target_timestamp)
            .expect("跨文件随机访问失败")
            .expect("未找到指定时间戳的数据包");

        assert_eq!(
            packet.packet.get_timestamp_ns(),
            target_timestamp,
            "跨文件访问时间戳不匹配"
        );

        // 验证数据包内容
        assert_eq!(
            packet.packet.packet_length() as usize,
            PACKET_SIZE,
            "跨文件访问数据包大小不匹配"
        );
    }

    // 测试跨文件时间范围读取
    let start_timestamp = written_timestamps[100]; // 第1个文件
    let end_timestamp = written_timestamps[700]; // 第4个文件

    let range_packets = reader
        .read_packets_by_time_range(
            start_timestamp,
            end_timestamp,
        )
        .expect("跨文件时间范围读取失败");

    assert_eq!(range_packets.len(), 601); // 100到700，包含两端

    // 验证跨文件读取的数据包时间戳顺序
    for i in 1..range_packets.len() {
        assert!(
            range_packets[i].packet.get_timestamp_ns()
                >= range_packets[i - 1]
                    .packet
                    .get_timestamp_ns(),
            "跨文件读取时间戳顺序错误"
        );
    }

    println!(
        "✅ 跨文件随机访问测试通过：{} 个文件，{} 个测试点",
        PACKET_COUNT.div_ceil(200),
        test_indices.len()
    );
}

#[test]
fn test_timestamp_index_edge_cases() {
    const TEST_NAME: &str =
        "test_timestamp_index_edge_cases";
    let dataset_path =
        setup_test_environment().expect("设置测试环境失败");

    // 清理测试数据集目录
    let test_dataset_path = dataset_path.join(TEST_NAME);
    clean_dataset_directory(&test_dataset_path)
        .expect("清理测试目录失败");

    const PACKET_COUNT: usize = 100;
    const PACKET_SIZE: usize = 32;

    // 创建数据集
    let mut writer = PcapWriter::new_with_config(
        &dataset_path,
        TEST_NAME,
        WriterConfig::default(),
    )
    .expect("创建PcapWriter失败");

    let mut written_timestamps = Vec::new();
    for i in 0..PACKET_COUNT {
        let packet =
            create_test_packet(i as u32, PACKET_SIZE)
                .expect("创建测试数据包失败");
        written_timestamps.push(packet.get_timestamp_ns());
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");

        // 添加小延迟确保时间戳递增
        std::thread::sleep(Duration::from_micros(1));
    }

    writer.finalize().expect("完成写入失败");

    // 测试边界情况
    let mut reader = PcapReader::new_with_config(
        &dataset_path,
        TEST_NAME,
        ReaderConfig::default(),
    )
    .expect("创建PcapReader失败");
    reader.initialize().expect("初始化Reader失败");

    // 测试最小时间戳
    let min_timestamp = written_timestamps[0];
    let packet = reader
        .read_packet_by_timestamp(min_timestamp)
        .expect("最小时间戳读取失败")
        .expect("未找到最小时间戳的数据包");
    assert_eq!(
        packet.packet.get_timestamp_ns(),
        min_timestamp
    );

    // 测试最大时间戳
    let max_timestamp =
        written_timestamps[PACKET_COUNT - 1];
    let packet = reader
        .read_packet_by_timestamp(max_timestamp)
        .expect("最大时间戳读取失败")
        .expect("未找到最大时间戳的数据包");
    assert_eq!(
        packet.packet.get_timestamp_ns(),
        max_timestamp
    );

    // 测试超出范围的时间戳
    let before_min = min_timestamp - 1000;
    let after_max = max_timestamp + 1000;

    let result = reader
        .read_packet_by_timestamp(before_min)
        .expect("读取超出范围的时间戳失败");
    assert!(result.is_none(), "应该返回None");

    let result = reader
        .read_packet_by_timestamp(after_max)
        .expect("读取超出范围的时间戳失败");
    assert!(result.is_none(), "应该返回None");

    // 测试空时间范围
    let empty_range = reader
        .read_packets_by_time_range(
            after_max,
            after_max + 1000,
        )
        .expect("空时间范围读取失败");
    assert!(
        empty_range.is_empty(),
        "空时间范围应该返回空结果"
    );

    // 测试单点时间范围
    let single_point = reader
        .read_packets_by_time_range(
            min_timestamp,
            min_timestamp,
        )
        .expect("单点时间范围读取失败");
    assert_eq!(
        single_point.len(),
        1,
        "单点时间范围应该返回1个数据包"
    );
    assert_eq!(
        single_point[0].packet.get_timestamp_ns(),
        min_timestamp
    );

    println!("✅ 时间戳索引边界情况测试通过");
}
